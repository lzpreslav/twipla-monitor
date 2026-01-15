{
  description = "Twipla event monitor with Slack notifications";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks.url = "github:cachix/git-hooks.nix";
    git-hooks.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      git-hooks,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        twipla-monitor = pkgs.callPackage ./nix/package.nix { };

        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nixfmt-rfc-style.enable = true;
            clippy = {
              enable = true;
              settings = {
                offline = false;
                denyWarnings = true;
              };
            };
          };
        };
      in
      {
        packages.default = twipla-monitor;

        checks = {
          inherit pre-commit-check;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            clippy
            openssl
            pkg-config
            rust-analyzer
            rustc
          ];
          shellHook = ''
            ${pre-commit-check.shellHook}
          '';
        };
      }
    )
    // {
      nixosModules.default = import ./nix/module.nix;
    };
}
