/** @type {import('tailwindcss').Config} */
import path from "path";

const ROOT_PATH = path.resolve(__dirname);

export const content = [
  `${ROOT_PATH}/styles/*.css`
];
export const theme = {
  extend: {},
};
export const plugins = [];
