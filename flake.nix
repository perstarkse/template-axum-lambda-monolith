{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.default = pkgs.mkShell {
        packages = [pkgs.bashInteractive pkgs.cargo-lambda pkgs.rustc pkgs.rustup pkgs.awscli2 pkgs.aws-sam-cli pkgs.clippy pkgs.openssl pkgs.pkg-config];
        SAM_CLI_BETA_RUST_CARGO_LAMBDA = 1;
        AWS_DEFAULT_REGION = "eu-north-1";
        IS_OFFLINE = true;
      };
    });
}
