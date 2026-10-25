# Third-party material

| what | where | licence |
|------|-------|---------|
| [Inter](https://rsms.me/inter/) variable font (via [Fontsource](https://fontsource.org)) | `crates/ui/fonts/InterVariable.woff2` | SIL Open Font License 1.1 |
| [Archivo](https://github.com/Omnibus-Type/Archivo) variable font (via Fontsource) | `crates/ui/fonts/ArchivoVariable.woff2` | SIL Open Font License 1.1 |
| [Lucide](https://lucide.dev) icons v0.469.0 | the `ICON_PATHS` table in `crates/ui/ts/ui.ts` | ISC |
| [oxc](https://oxc.rs), [tiny_http](https://github.com/tiny-http/tiny-http), serde_json | Rust dependencies (see `Cargo.lock`) | MIT / Apache-2.0 |

Data is fetched at run time from Open-Meteo (weather), the National Grid ESO Carbon Intensity API, the Octopus Energy API and Google
Calendar, under their own terms; nothing from them is stored in this repository.

Berkeley Mono (the default monospace) is a licensed typeface owned by its licensee. Only an [age](https://age-encryption.org)-encrypted copy,
`crates/ui/fonts/berkeley.woff2.age`, is in this repository; it is unusable without the key, which is not published. The server decrypts it
at startup (in memory) if `secrets/font.key` exists. Without the key the UI uses the system monospace. Do not ask for the key: the
typeface must be licensed from [Berkeley Graphics](https://berkeleygraphics.com/typefaces/berkeley-mono/).
