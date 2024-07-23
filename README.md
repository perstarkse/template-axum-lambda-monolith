# AWS Rust Monolith API

## Helpful hints

sam build from base dir to generate the build
sam local start-api --env-vars local-env.json to start a local api instance
cargo deny --log-level error check to check licenses for dependencies
sam deploy to deploy

## Features

DynamoDB set up, single table for now

## Why?

Designed as a monolith to ease transitioning to another hosting solution
Rust is fast anyways
This design choice could be redone if application grows significantly
