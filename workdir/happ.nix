{ inputs, ... }:

{
  perSystem = { inputs', lib, self', system, ... }: {
    packages.private_event_sourcing_test_happ =
      inputs.holochain-utils.outputs.builders.${system}.happ {
        happManifest = ./happ.yaml;

        dnas = {
          # Include here the DNA packages for this hApp, e.g.:
          # my_dna = inputs'.some_input.packages.my_dna;
          # This overrides all the "bundled" properties for the hApp manifest 
          private_event_sourcing_test =
            self'.packages.private_event_sourcing_test_dna;
        };
      };
  };
}
