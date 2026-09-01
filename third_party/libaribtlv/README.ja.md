# libaribtlv

[English](README.md) | 日本語

`libaribtlv` は、復号済み ARIB MMT/TLV ストリームをインクリメンタルに
解析するライブラリです。ネイティブ受信機、コマンドラインツール、
WebAssembly アダプターで共有する次のコア機能を提供します。

- TLV 再同期、圧縮 IP コンテキスト、MMTP 分割・集約処理
- PA/M2/MPT、MH-AIT、EIT、SDT、TOT、EMT、B60 データ伝送シグナリング
- HEVC、AAC-LATM/LOAS、ARIB STD-B62 TTML アクセスユニット
- ARIB-HTML5 アプリケーションリソースの組み立てとメモリ内ストア
- 録画インデックス、シークポイント、長さ計測、Range ベースの probe

プレイヤーのセッション制御、MSE/fMP4 remux、JavaScript binding、一般的な再生
command、ブラウザ demo は sibling の `tlvdemux` プロジェクトに置きます。

## ビルドとテスト

```sh
nix-shell
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
ctest --test-dir build --output-on-failure
```

既定では共有ライブラリ `libaribtlv.so.0` または
`libaribtlv.0.dylib` を生成します。静的ライブラリが必要な場合は
`-DBUILD_SHARED_LIBS=OFF` を指定してください。標準ライブラリ以外の依存は
Zlib のみで、Emscripten では組み込みの zlib port を利用します。

```sh
cmake --install build --prefix /desired/prefix
```

利用側は `pkg-config --cflags --libs libaribtlv`、または
`find_package(aribtlv CONFIG REQUIRED)` と `aribtlv::aribtlv` を使用します。
静的リンクでは pkg-config に `--static` を追加してください。

## 安定 C API

C API は FFmpeg などのメディアフレームワーク向けの安定した連携境界です。
C++ 型や FFmpeg 型を公開しません。

```c
#include <aribtlv/aribtlv.h>

static void on_access_unit(void *opaque, const aribtlv_access_unit *unit) {
    /* callback 後も必要なデータはここでコピーします。 */
}

aribtlv_callbacks callbacks;
aribtlv_callbacks_init(&callbacks);
callbacks.on_access_unit = on_access_unit;

aribtlv_demuxer *demuxer = aribtlv_demuxer_create(NULL, &callbacks, NULL);
aribtlv_demuxer_push(demuxer, data, size);
aribtlv_demuxer_flush(demuxer);
aribtlv_demuxer_destroy(demuxer);
```

解析と callback は同期実行され、`aribtlv_demuxer_push()` の返却後に入力
ポインターは保持されません。callback に渡される構造体とバイト列はその
callback 中だけ有効な view なので、キューに残す場合はコピーしてください。
version 付き設定・callback 構造体は対応する `_init()` で初期化します。

字幕トラックでは `aribtlv_track_info.subtitle` から B60 の追加字幕情報を
取得できます。各 TTML `aribtlv_access_unit` には、該当する timing / operation /
display / compression mode、MPU sequence、reference-start の media timestamp、
同一 MPU の resource view も含まれます。これらは callback 中だけ有効な view で、
C++ ABI に依存しない FFmpeg などの adapter から利用できます。

同じ安定 C API から、C++ ABI を使わずに HLG-to-SDR LUT を取得できます。

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

buffer の所有権は caller にあります。内容は 0..1 の RGB float triplet で、red、
green、blue の順に座標が変化します。Iridas `.cube` と FFmpeg `lut3d` が読む順序と
同じです。buffer が不足する場合は部分出力せず
`ARIBTLV_ERROR_BUFFER_TOO_SMALL` を返します。

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

`push()` は任意のチャンク境界を同期的に処理し、入力ポインターを保持しません。
同じソース内のシークには `reposition()`、ソース交換には `reset()`、実際の
入力境界または EOF には `flush()` を使用します。

C++ API は C++20 が必要です。動的リンクする C++ consumer は互換性のある
コンパイラと C++ 標準ライブラリを使用してください。toolchain に依存しない
安定 ABI が必要な場合は C API を使用します。

## FFmpeg 連携とライセンス

FFmpeg 側には `AVInputFormat` の薄い adapter だけを置き、解析は install 済み
C API に委譲します。`libaribtlv` は独立した MIT ライセンスであり、LGPL/GPL
の FFmpeg build からリンクされても本 repository のライセンスは変わりません。
FFmpeg のコードをこのライブラリへコピーすると元のライセンスが残るため、
core 実装には取り込みません。

FFmpeg filter は C API の float buffer を直接利用できます。LUT が想定する HLG
RGB code value を渡す色変換と、出力を SDR として mark する処理は FFmpeg filter
graph 側の責任です。demuxer は復号後 pixel を処理しません。

## ライセンス

MIT
