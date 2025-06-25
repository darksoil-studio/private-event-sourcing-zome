{ inputs, ... }:

{
  perSystem = { inputs', system, ... }: {
    builders.encrypted_links = { private_event_sourcing_zome_name }:
      inputs.holochain-nix-builders.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;

        zomeEnvironmentVars = {
          PRIVATE_EVENT_SOURCING_ZOME = private_event_sourcing_zome_name;
        };
      };
  };
}

