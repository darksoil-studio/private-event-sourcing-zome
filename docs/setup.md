# Setup

> [!WARNING]
> This guide assumes that you have scaffolded a hApp with the [TNESH stack template](https://darksoil.studio/tnesh-stack).

1. Run this to scaffold this zome in your hApp:

```bash
nix run github:darksoil-studio/private-event-sourcing-zome#scaffold
```

This will do the following:
  - Add the `github:darksoil-studio/private-event-sourcing-zome` flake input in your `flake.nix`.
  - Add the `private_event_sourcing` coordinator and integrity zome packages to the `dna.nix` that you select.
  - Add the UI package for `@darksoil-studio/private-event-sourcing-zome` as a dependency of your UI package.

That's it! You have now integrated the `private_event_sourcing` coordinator and integrity zomes and their UI into your app!


