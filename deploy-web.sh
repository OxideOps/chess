#!/usr/bin/env bash

ORG=event-horizon-technologies
REPO=chess-web
MSG=$(git log -1 --pretty=%B)

rm -rf /tmp/$REPO
git clone git@github.com:$ORG/$REPO.git /tmp/$REPO
trunk build
rm -rf /tmp/$REPO/docs
mv dist /tmp/$REPO/docs
cd /tmp/$REPO
git add .
git commit -m"$MSG"
git push
