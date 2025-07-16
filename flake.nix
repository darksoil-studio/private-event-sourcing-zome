{
  description = "Template for Holochain app development";

  inputs = {
    holochain-utils.url = "github:darksoil-studio/holochain-utils/main-0.5";
    nixpkgs.follows = "holochain-utils/nixpkgs";

    linked-devices-zome.url =
      "github:darksoil-studio/linked-devices-zome/main-0.5";
    linked-devices-zome.inputs.holochain-utils.follows = "holochain-utils";
  };

  nixConfig = {
    extra-substituters = [
      "https://holochain-ci.cachix.org"
      "https://darksoil-studio.cachix.org"
    ];
    extra-trusted-public-keys = [
      "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8="
      "darksoil-studio.cachix.org-1:UEi+aujy44s41XL/pscLw37KEVpTEIn8N/kn7jO8rkc="
    ];
  };

  outputs = inputs:
    inputs.holochain-utils.inputs.holonix.inputs.flake-parts.lib.mkFlake {
      inherit inputs;
    } {
      imports = [
        ./zomes/integrity/example/zome.nix
        ./zomes/coordinator/example/zome.nix
        ./zomes/integrity/encrypted_links/zome.nix
        ./zomes/coordinator/encrypted_links/zome.nix
        # Just for testing purposes
        ./workdir/dna.nix
        ./workdir/happ.nix
        inputs.holochain-utils.outputs.flakeModules.builders
      ];

      systems =
        builtins.attrNames inputs.holochain-utils.inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.holochain-utils.devShells.synchronized-pnpm
            inputs'.holochain-utils.devShells.default
          ];

          packages = [
            inputs'.holochain-utils.packages.holochain
            inputs'.holochain-utils.packages.hc-scaffold-zome
            inputs'.holochain-utils.packages.hc-playground
          ];
        };
        devShells.npm-ci = inputs'.holochain-utils.devShells.synchronized-pnpm;

        packages.scaffold = pkgs.symlinkJoin {
          name = "scaffold-remote-zome";
          paths = [ inputs'.holochain-utils.packages.scaffold-remote-zome ];
          buildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            wrapProgram $out/bin/scaffold-remote-zome \
              --add-flags "private-event-sourcing-zome \
                --integrity-zome-name private_event_sourcing_integrity \
                --coordinator-zome-name private_event_sourcing \
                --remote-zome-git-url github:darksoil-studio/private-event-sourcing-zome \
                --remote-npm-package-name @darksoil-studio/private-event-sourcing-zome \
                --remote-zome-git-branch main-0.5 \
                --context-element private-event-sourcing-context \
                --context-element-import @darksoil-studio/private-event-sourcing-zome/dist/elements/private-event-sourcing-context.js" 
          '';
        };
      };
    };
}
