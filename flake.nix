{
  description = "Twipla event monitor with Slack notifications";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        twipla-monitor = pkgs.callPackage ./nix/package.nix { };
      in
      {
        packages.default = twipla-monitor;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            clippy
            openssl
            pkg-config
            rust-analyzer
            rustc
          ];
        };
      }
    )
    // {
      nixosModules.default = import ./nix/module.nix;
    };
}
