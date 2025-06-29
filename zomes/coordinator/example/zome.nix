{ inputs, ... }:

{
  perSystem = { inputs', system, ... }: {
    packages.example =
      inputs.holochain-nix-builders.outputs.builders.${system}.rustZome {
        workspacePath = inputs.self.outPath;
        crateCargoToml = ./Cargo.toml;
        zomeEnvironmentVars = { ASYNC_MESSAGE_ZOME = "encrypted_links"; };
      };
  };
}

