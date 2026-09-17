//! Grading a coach answer against what the coach was given.
//!
//! The deterministic check (`Prompt::check`) only catches moves and pieces
//! the prompt never showed. This asks a second model call to read the same
//! facts and lines the coach had, quote anything in the answer they don't
//! support, and score the answer. It is the eval's yardstick, not something
//! the server does: a judge is another language model and can be wrong, so
//! the run prints its quotes for a human to weigh.

use serde::{Deserialize, Serialize};
use server::coach::Usage;

/// Grading is a checking job against material the judge is handed, and most
/// of its cost is thinking, so it runs at low effort on the middle model:
/// Haiku 4.5 hands out 5s (it disagreed with Opus by 1.12 on accuracy, Sonnet
/// by 0.48), and Opus costs more than the answer it grades.
/// `CHESS_JUDGE_MODEL` picks another.
pub const DEFAULT_JUDGE_MODEL: &str = "claude-sonnet-5";
/// Models that take `output_config.effort`; Haiku 4.5 rejects it.
const EFFORT: &[&str] = &[
    "claude-opus-5",
    "claude-opus-4-8",
    "claude-sonnet-5",
    "claude-fable-5-1",
    "claude-fable-5",
];
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

const SYSTEM: &str = "You grade a chess coach's explanations for a club player. You are given \
the material the coach was given (facts computed from the board, and a chess engine's lines \
with its evaluations) and the coach's answer. Judge the answer only against that material: it \
is the only source of truth, and you must not add chess judgements of your own. Quote every \
statement in the answer that the material doesn't support or that contradicts it, exactly as \
written, and say why. A plausible-sounding claim with nothing behind it in the material counts \
as unsupported; a fair paraphrase of the material does not.\n\
Judge only what the answer says about this position, this line and these evaluations. A \
general chess principle (\"bring the king up\", \"develop before attacking\"), the name of an \
opening or a mating pattern, and encouragement are a coach's job: leave those out, unless \
they contradict the material or are dressed up as a fact about this position.\n\
Then score, 1 to 5:\n\
accuracy: 5 nothing unsupported; 4 one harmless imprecision; 3 one unsupported claim that \
wouldn't mislead; 2 a wrong claim about the position; 1 several wrong claims, or the main \
point is wrong.\n\
clarity: 5 a club player follows it in one read, moves named with their squares; 3 followable \
but heavy going; 1 confusing or jargon-ridden.\n\
usefulness: 5 teaches the idea that matters here and what to aim for; 3 describes the moves \
without the idea; 1 nothing a student can use.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Unsupported {
    /// The statement, exactly as the answer wrote it.
    pub quote: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Judgement {
    pub accuracy: u8,
    pub clarity: u8,
    pub usefulness: u8,
    pub unsupported: Vec<Unsupported>,
    /// A sentence for a human reading the run.
    pub notes: String,
}

impl Judgement {
    /// The three scores added up, out of 15.
    pub fn total(&self) -> u32 {
        u32::from(self.accuracy) + u32::from(self.clarity) + u32::from(self.usefulness)
    }
}

/// What the model must answer with; scores are enums because the API's
/// structured outputs don't take numeric ranges.
fn schema() -> serde_json::Value {
    let score = serde_json::json!({ "type": "integer", "enum": [1, 2, 3, 4, 5] });
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["accuracy", "clarity", "usefulness", "unsupported", "notes"],
        "properties": {
            "accuracy": score,
            "clarity": score,
            "usefulness": score,
            "unsupported": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["quote", "why"],
                    "properties": { "quote": { "type": "string" }, "why": { "type": "string" } }
                }
            },
            "notes": { "type": "string" }
        }
    })
}

pub struct Judge {
    client: reqwest::Client,
    key: String,
    pub model: String,
}

impl Judge {
    pub fn new(key: String, model: String) -> Judge {
        Judge {
            client: reqwest::Client::new(),
            key,
            model,
        }
    }

    /// Grade `answer` against the `prompt` the coach was given.
    pub async fn grade(&self, prompt: &str, answer: &str) -> Result<(Judgement, Usage), String> {
        let user = format!(
            "=== What the coach was given ===\n{prompt}\n\n=== The coach's answer ===\n{answer}"
        );
        let mut output_config = serde_json::json!({
            "format": { "type": "json_schema", "schema": schema() }
        });
        // Grading is checking, not thinking hard: the difference is most of
        // what a run costs.
        if EFFORT.contains(&self.model.as_str()) {
            output_config["effort"] = serde_json::json!("low");
        }
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4000,
            "fallbacks": "default",
            "system": SYSTEM,
            "output_config": output_config,
            "messages": [{ "role": "user", "content": user }],
        });
        let response = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("anthropic-beta", FALLBACK_BETA)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request: {e}"))?;
        let status = response.status();
        let parsed: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("response: {e}"))?;
        if !status.is_success() {
            return Err(format!("{status}: {parsed}"));
        }
        if parsed["stop_reason"] != "end_turn" {
            return Err(format!("stopped: {}", parsed["stop_reason"]));
        }
        let text: String = parsed["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b["type"] == "text")
                    .filter_map(|b| b["text"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let usage: Usage = serde_json::from_value(parsed["usage"].clone()).unwrap_or_default();
        let judgement = serde_json::from_str(&text).map_err(|e| format!("{e}: {text}"))?;
        Ok((judgement, usage))
    }
}
