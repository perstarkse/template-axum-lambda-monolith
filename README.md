# **AWS Rust Monolith API**

### Overview

This project is a serverless API built with Rust and Axum, deployed as a single Lambda function behind an API Gateway with proxy integration. It uses AWS Cognito for authentication, with middleware handling auth logic.

### Features

- AWS Cognito integration with middleware-based authentication
- DynamoDB setup with CRUD operations for a template `Item` type
- Example routes demonstrating how to build upon this template
- Deployed as a single Lambda function with API Gateway proxy integration

### Development

- Build: `sam build` from the base directory
- Run local API: `sam local start-api --env-vars local-env.json`
- Check dependency licenses: `cargo deny --log-level error`
- Lint: `cargo clippy`
- Run unit tests: `cargo test`
- Deploy: `sam deploy`

### Running Locally

To run the API locally, you'll need to set the required environment variables. One way to do this is to create a `local-env.json` file with the necessary variables. You can copy the `local-env.json.example` file and update it with your own values.

### Design Notes

This project is designed as a monolith to facilitate easy transition to alternative hosting solutions. Rust's performance capabilities make this design choice suitable for now. If the application grows significantly, reassessing this architecture may be necessary.

### Architecture

The API is deployed with the following AWS resources:

- AWS Cognito User Pool and Client
- API Gateway with proxy integration
- Lambda function with Rust runtime
- DynamoDB table for data storage
