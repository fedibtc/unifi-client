{
  description = "Node.js TypeScript development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            nodejs_22
            nodePackages.typescript
            nodePackages.typescript-language-server
            nodePackages.pnpm
          ];

          shellHook = ''
            echo "Node.js TypeScript development environment loaded!"
            echo "Node.js $(node --version)"
            echo "pnpm $(pnpm --version)"
            echo "TypeScript $(tsc --version)"
          '';
        };
      }
    );
}