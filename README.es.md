# @kryxjs/ogg

[English](./README.md) · **Español**

[![CI](https://github.com/Brashkie/kryx-ogg/actions/workflows/ci.yml/badge.svg)](https://github.com/Brashkie/kryx-ogg/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/@kryxjs/ogg)](https://www.npmjs.com/package/@kryxjs/ogg)
[![Licencia](https://img.shields.io/badge/licencia-Apache--2.0-blue)](./LICENSE)

Un lector y escritor pequeño y sin dependencias para el formato contenedor
**Ogg** (RFC 3533), escrito en Rust puro con un SDK de TypeScript. Parte del
ecosistema multimedia [Kryx](https://github.com/Brashkie).

> **Estado: estable (0.1.0).** Lee y escribe Ogg desde Rust y JavaScript:
> páginas, validación de CRC-32, reensamblado de paquetes, streams lógicos y un
> escritor — todo con cero dependencias en runtime. Se combina con
> `@kryxjs/codecs-opus` para leer y escribir archivos `.opus` reales de punta a
> punta.

## Filosofía

`@kryxjs/ogg` conoce **solo Ogg**. Entrega los bytes crudos de cada paquete más
su número de serial y su granule position (sin interpretar). Lo que un paquete
*significa* — un `OpusHead`, un header de Vorbis, un frame de audio — es trabajo
del códec, una capa más arriba. El mismo lector sirve para Opus, Vorbis, FLAC y
Theora sin conocer ninguno de ellos.

Este es el primer paquete 100% propio y sin C de Kryx: sin librería vendorizada,
sin Zig, solo Rust. Es lo que hace que `@kryxjs/codecs-opus` sea autosuficiente
de punta a punta — leer y escribir archivos `.opus` reales sin ninguna
herramienta externa.

## Crates

- **`ogg-core`** — el motor en Rust puro. Cero dependencias. Reutilizable desde
  WASM, una CLI, Tauri, otros lenguajes o tests en Rust puro — no tiene acople
  con Node.
- **`ogg-node`** — el puente napi hacia Node.js.

## Roadmap

| Hito | Alcance |
|------|---------|
| **M1** ✅ | Lectura en `ogg-core`: páginas, validación CRC-32, reensamblado de paquetes, streams lógicos |
| **M2** ✅ | API pública + napi (`OggReader` → `streams()` → `packets()`) + SDK de TS |
| **M3** ✅ | Escritura en `ogg-core`: `OggWriter` (páginas válidas, CRC, mux de un stream) |
| **M4** ✅ | Integración con Opus (`OpusStream` en `codecs-opus` lee `OpusHead`/`OpusTags`) |
| **M5** ✅ | Estable 0.1.0 |

Mirá **[ROADMAP.md](./ROADMAP.md)** para lo que viene: completar el formato
(lectura incremental, seeking, streams encadenados, muxing), rendimiento
(zero-copy, CRC-32 con SIMD, benchmarks vs libogg/ffmpeg) y los diferenciadores
(lectura robusta/con recuperación, diagnósticos, WASM, una CLI, fuzzing).

## Referencias

- RFC 3533 — El formato de encapsulado Ogg
- RFC 7845 — Encapsulado Ogg para el códec de audio Opus

## Uso

### Lectura

```ts
import { readFile } from 'node:fs/promises'
import { OggReader } from '@kryxjs/ogg'

const bytes = await readFile('audio.opus')
const reader = new OggReader(bytes)

for await (const stream of reader.streams()) {
  console.log('stream lógico', stream.serial)
  for await (const packet of stream.packets()) {
    // packet.data       → Buffer (bytes crudos del paquete)
    // packet.serial     → number (serial del stream lógico)
    // packet.granulePosition → bigint | null (definido por el códec; sin interpretar)
  }
}
```

### Escritura

```ts
import { OggWriter } from '@kryxjs/ogg'

const bytes = new OggWriter(serial)
  .write(packetBytes, 960n)   // paquete + granule position (bigint o number)
  .write(moreBytes, 1920n)
  .finish()                    // → Buffer con el stream Ogg completo
```

El escritor se encarga de toda la mecánica — segmentar paquetes grandes,
empaquetar páginas, los flags BOS/EOS, los números de secuencia y los CRC. Vos
das los paquetes y las granule positions; nunca construís una página a mano.

`@kryxjs/ogg` devuelve y acepta **paquetes crudos** — no los interpreta. Para
convertir los paquetes de un archivo Ogg-Opus en audio, combinalo con
`@kryxjs/codecs-opus`, cuyo `OpusStream` lee los headers `OpusHead`/`OpusTags`
sobre este paquete.

La API es una *API de streaming sobre un motor eager*: hoy la capa nativa
parsea todo el buffer de una y el SDK va entregando a través de iteradores
asíncronos. La forma pública es la definitiva — un motor incremental futuro
puede reemplazar los internos sin ningún cambio visible para el usuario.

## Licencia

[Apache-2.0](./LICENSE) © Brashkie
