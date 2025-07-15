{ inputs, ... }:

{
  perSystem = { inputs', system, ... }: {
    packages.encrypted_links =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

