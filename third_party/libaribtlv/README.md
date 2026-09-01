# libaribtlv

English | [日本語](README.ja.md)

`libaribtlv` is a library for incrementally decoding already-descrambled
ARIB MMT/TLV streams. It owns the transport and broadcast-domain core shared by
native receivers, command-line tools, and WebAssembly adapters:

- TLV resynchronization, compressed-IP contexts, and MMTP fragmentation;
- PA/M2/MPT, MH-AIT, EIT, SDT, TOT, EMT, and B60 data-transmission signalling;
- HEVC, AAC-LATM/LOAS, and ARIB STD-B62 TTML access units;
- ARIB-HTML5 application-resource assembly and an in-memory resource store;
- recording indexes, seek points, duration tracking, and range-based probing.

Player session policy, MSE/fMP4 remuxing, JavaScript bindings, general playback
commands, and browser demos live in the sibling `tlvdemux` project.

## Build and test

```sh
nix-shell
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
ctest --test-dir build --output-on-failure
```

Shared libraries are enabled by default and produce `libaribtlv.so.0` or
`libaribtlv.0.dylib`. Set `-DBUILD_SHARED_LIBS=OFF` for `libaribtlv.a`.
Zlib is the only non-standard runtime dependency. Emscripten builds use its
built-in zlib port.

Install the library, headers, CMake package, and `libaribtlv.pc` with:

```sh
cmake --install build --prefix /desired/prefix
```

Consumers can use either `pkg-config --cflags --libs libaribtlv`, or
`find_package(aribtlv CONFIG REQUIRED)` and link `aribtlv::aribtlv`. Add
`--static` to the pkg-config command when linking `libaribtlv.a`.

## Stable C API

The C API is the stable integration boundary for FFmpeg and other media
frameworks. It does not expose C++ types or FFmpeg types.

```c
#include <aribtlv/aribtlv.h>

static void on_access_unit(void *opaque, const aribtlv_access_unit *unit) {
    /* Copy data that must outlive this callback. */
}

aribtlv_callbacks callbacks;
aribtlv_callbacks_init(&callbacks);
callbacks.on_access_unit = on_access_unit;

aribtlv_demuxer *demuxer = aribtlv_demuxer_create(NULL, &callbacks, NULL);
aribtlv_demuxer_push(demuxer, data, size);
aribtlv_demuxer_flush(demuxer);
aribtlv_demuxer_destroy(demuxer);
```

Parsing and callbacks are synchronous. Input bytes are not retained after
`aribtlv_demuxer_push()` returns. Structures and byte strings passed to a
callback are views valid only for that callback; copy anything that must be
queued by the consumer. Initialize versioned configuration and callback
structures with their `_init()` functions.

For a subtitle track, `aribtlv_track_info.subtitle` exposes the complete B60
additional subtitle information. Each TTML `aribtlv_access_unit` also carries
the applicable timing, operation, display, and compression modes, the MPU
sequence, the reference-start media timestamp, and same-MPU resource views.
These fields are callback-lifetime views and are intended for adapters such as
FFmpeg without requiring C++ ABI access.

The same stable C API exposes the HLG-to-SDR LUTs without requiring a C++ ABI:

```c
aribtlv_hlg_sdr_lut_info info;
aribtlv_hlg_sdr_lut_describe(
    ARIBTLV_HLG_SDR_LUT_BT2446_PROTOTYPE, &info);
float *rgb = malloc(info.rgb_float_count * sizeof(*rgb));
aribtlv_hlg_sdr_lut_generate(
    ARIBTLV_HLG_SDR_LUT_BT2446_PROTOTYPE,
    rgb,
    info.rgb_float_count);
```

The caller owns the buffer. It contains normalized RGB float triplets with red
changing fastest, then green, then blue. This is the ordering consumed by
Iridas `.cube` files and FFmpeg's `lut3d` filter. An undersized buffer is
rejected with `ARIBTLV_ERROR_BUFFER_TOO_SMALL` without partial output.

## C++ API

```cpp
#include <aribtlv/demuxer.hpp>

class Receiver final : public aribtlv::Sink {
public:
    void onService(const aribtlv::ServiceInfo&) override;
    void onTrack(const aribtlv::TrackInfo&) override;
    void onAccessUnit(aribtlv::AccessUnit&&) override;
    void onError(const aribtlv::Error&) override;
};

Receiver receiver;
aribtlv::Demuxer demuxer(receiver);
demuxer.push(data, size);
demuxer.flush();
```

`push()` consumes arbitrary chunk boundaries synchronously and does not retain
the input pointer. Use `reposition()` for a seek in the same source, `reset()`
when replacing the source, and `flush()` at a true input boundary or EOF.

Application resources can be received directly through `Sink`, or assembled
explicitly with `ApplicationResourceAssembler`. `Limits` bounds parser and
resource memory, and can disable built-in application-resource collection when
an adapter needs to drain that work asynchronously.

The C++ API requires C++20. Dynamically linked C++ consumers must use a
compatible compiler and C++ standard library; consumers that need a stable
toolchain-neutral ABI should use the C API.

## FFmpeg integration and licensing

An FFmpeg integration should keep only an `AVInputFormat` adapter in the FFmpeg
tree and use the installed C API for parsing. `libaribtlv` is independently MIT
licensed; linking it from an LGPL/GPL FFmpeg build does not change the license
of this repository. Code copied from FFmpeg into this library would retain its
original license and therefore must not be used for the core implementation.

An FFmpeg filter can consume the C API's float buffer directly. The filter graph
remains responsible for presenting the LUT with the expected HLG RGB code
values and for marking the result as SDR; the demuxer does not process decoded
pixels.

## License

MIT
