{ inputs, ... }:

{
  perSystem = { inputs', self', lib, system, ... }: {
    packages.private_event_sourcing_test_dna =
      inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
          # This overrides all the "bundled" properties for the DNA manifest
          example = self'.packages.example;
          example_integrity = self'.packages.example_integrity;
        };
      };
  };
}

