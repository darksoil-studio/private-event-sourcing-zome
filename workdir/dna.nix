{ inputs, ... }:

{
  perSystem = { inputs', self', lib, system, ... }: {
    packages.private_event_sourcing_test_dna =
      inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
          linked_devices_integrity = inputs'.linked-devices-zome.packages.linked_devices_integrity;
          linked_devices = inputs'.linked-devices-zome.packages.linked_devices;
          # This overrides all the "bundled" properties for the DNA manifest
          example = self'.packages.example;
          example_integrity = self'.packages.example_integrity;
        };
      };
  };
}

