{ inputs, ... }:

{
  perSystem = { inputs', system, ... }: {
    packages.example_integrity =
      inputs.holochain-utils.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
      };
  };
}

