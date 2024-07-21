# NxtPoll API

NxtPoll API is a secure, serverless Rust application designed to handle polling operations. It leverages AWS Lambda and Cognito for authentication, providing a robust backend for polling applications.

### Checklist

**Set Up AWS Cognito User Pool**

- \[ \] Create Cognito User Pool via AWS Console.
- \[ \] Configure App Client for the User Pool.

**Configure API Gateway with Cognito Authorizer**

- \[ \] Create Cognito User Pool Authorizer in API Gateway.
- \[ \] Attach the authorizer to the API Gateway methods.

**Implement Rust Application**

- \[x\] Set up a Rust project using Axum and Lambda.
- \[ \] Add JWT token parsing and verification in your handler.
- \[ \] Create routes and ensure user authentication.

**Fetch JWKS Keys**

- \[ \] Implement a function to fetch JWKS keys from Cognito.

**Generate OpenAPI Specification**

- \[ \] Add `utoipa` and `utoipa-swagger-ui` dependencies for OpenAPI documentation.
- \[ \] Annotate your Rust handlers with OpenAPI annotations.
- \[ \] Generate OpenAPI spec using `utoipa`.

**Serve OpenAPI Documentation**

- \[ \] Create an endpoint to serve the OpenAPI JSON.
- \[ \] Set up Swagger UI to serve the documentation using the generated OpenAPI spec.

**Update SAM Template**

- \[x\] Define Lambda function, API Gateway, and DynamoDB table.
- \[ \] Configure environment variables for JWKS URL and Cognito Client ID.
- \[ \] Set up API Gateway Cognito Authorizer.
- \[ \] Include OpenAPI spec location in the API Gateway definition.

**Add Clippy to Rust Project**

- \[ \] Add `clippy` as a development dependency in `Cargo.toml`.
- \[ \] Create a script to run Clippy checks.

**Update Deployment Scripts**

- \[ \] Modify deployment script to include Clippy checks.
- \[ \] Generate OpenAPI spec before building.
- \[ \] Build Rust project for Lambda.
- \[ \] Package and deploy using AWS SAM.

**Set Up GitHub Actions for CI/CD**

- \[ \] Define workflow for building, linting, and deploying.
- \[ \] Include steps for Clippy checks, building, generating OpenAPI spec, and deploying with SAM.

## Features

- Serverless architecture using AWS Lambda
- Secure authentication with AWS Cognito
- RESTful API endpoints for various polling operations
- Health check endpoint for monitoring
- Configurable environment settings
- Middleware for request authentication
- Caching mechanism for Cognito public keys

## Installation

1. Ensure you have Rust and Cargo installed on your system.
1. Clone the repository:
   ```
   git clone https://github.com/yourusername/nxtpoll-api.git
   cd nxtpoll-api
   ```
1. Install the AWS SAM CLI following the [official documentation](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-sam-cli-install.html).

## Usage

To run the application locally:

```bash
sam build
sam local start-api
```

To deploy to AWS:

```bash
sam deploy --guided
```

## API Endpoints

- `GET /`: Root endpoint
- `GET /foo`: Foo endpoint
- `POST /foo`: Post to foo endpoint
- `POST /foo/:name`: Post to foo with name parameter
- `GET /parameters`: Get request parameters
- `GET /health`: Health check endpoint

## Configuration

The application uses environment variables for configuration. Set the following variables:

- `ENVIRONMENT`: The current environment (e.g., "development", "production")
- `COGNITO_USER_POOL_ID`: Your AWS Cognito User Pool ID
- `COGNITO_APP_CLIENT_ID`: Your AWS Cognito App Client ID
- `AWS_REGION`: The AWS region for your resources

## Development

To contribute to the project:

1. Fork the repository
1. Create a new branch for your feature
1. Make your changes and write tests
1. Submit a pull request with a clear description of your changes

## Testing

Run the tests using:

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

- [Axum](https://github.com/tokio-rs/axum) for the web framework
- [AWS SAM](https://aws.amazon.com/serverless/sam/) for serverless deployment
- [jsonwebtoken](https://github.com/Keats/jsonwebtoken) for JWT handling
