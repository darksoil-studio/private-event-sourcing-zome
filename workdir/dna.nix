{ inputs, ... }:

{
  perSystem = { inputs', self', lib, system, ... }: {
    packages.private_event_sourcing_test_dna =
      inputs.holochain-utils.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
          linked_devices_integrity =
            inputs'.linked-devices-zome.packages.linked_devices_integrity;
          linked_devices = inputs'.linked-devices-zome.packages.linked_devices;

          example = self'.packages.example;
          example_integrity = self'.packages.example_integrity;

          encrypted_links_integrity = self'.packages.encrypted_links_integrity;
          encrypted_links = self'.packages.encrypted_links;
        };
      };
  };
}

