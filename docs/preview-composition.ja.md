[简体中文](preview-composition.md) · [繁體中文](preview-composition.zh-TW.md) · [日本語](preview-composition.ja.md) · [English](preview-composition.en.md)

> **規範上の注意:** 簡体字中国語版だけが正文です。他言語版は同期翻訳であり、表現に曖昧さや食い違いがある場合は簡体字中国語版を優先します。

# プレビュー合成方式

Windows のプレビューには、プロセス内の `libmpv-render` を使います。ResubWinny は `mpv.exe` を起動せず、JSON named pipe も使いません。macOS と Linux は同じ字幕モデルを共有しますが、ネイティブプレビューのバックエンドはまだ実装していません。

libmpv の映像と Rust の字幕は、専用レンダースレッド上のオフスクリーン FBO で合成します。親を持たず、画面外に置かれ、表示されることのない WGL host は、OpenGL device context を提供するためだけに存在します。映像の表示やアプリウィンドウの重なり、移動、サイズ変更には関与しません。現在の経路では、可視または子の HWND、`wid`、`overlay-add`、ソースごとのフォールバックを使いません。

合成済みの RGBA フレームは、順番に使い回す PBO から読み戻します。映像が libmpv の音声より常に 2 フレーム遅れないよう、各フレームは投入直後に map します。3 個の WebView2 SharedBuffer ピクセルスロットが、プレイヤー部品内の WebGL Canvas にフレームを渡します。制御スロットが持つのは連番、寸法、スロット状態だけです。Canvas は最新の準備済みフレームを取得し、アップロード済みまたは古いスロットを解放します。フレームごとのピクセルは Tauri command、event、COM copy を通りません。

ウィンドウ移動、ペインの折りたたみ、DPI の変化が影響するのは Canvas の DOM layout だけです。ResizeObserver が部品の物理ピクセル寸法をバックエンドへ送り、バックエンドはオフスクリーン FBO だけをリサイズします。映像とプレイヤー操作部は常に同じ WebView content layer にあり、ネイティブウィンドウの座標を追跡しません。共有サーフェスの上限は 1920×1080 で、それより大きなソースは libmpv が部品の実寸へ縮小します。

`get_preview_capabilities` が報告するのは、現在の `libmpv-render` 経路だけです。`get_preview_render_diagnostics` は、render、present、drop のフレーム数、サーフェス寸法と頻度、字幕テクスチャ操作、映像比率、デコーダーポリシー、直近のエラーを返します。`render_at(time)` は引き続き、指定時刻の bounded archive snapshot に使います。`sync_preview_overlay` は libmpv のメディア時刻からプロジェクト時刻を求めて Rust の字幕テクスチャを更新します。WebView 側に別の再生時計や字幕レイアウト処理は持ちません。

レンダースレッドは `hwdec=auto-safe` を要求し、取得できる場合は libmpv が選択した `hwdec-current` を報告します。互換性のある copy-back hardware decode は利用できますが、D3D/ANGLE の zero-copy を保証するものではありません。プレビュー停止時とアプリ終了時には、レンダースレッドを停止して join し、WebView UI STA 上で SharedBuffer を解放してから、非表示の WGL host を破棄します。

`scripts/validate-preview.ps1` は、`ARIB_FIXTURE_DIR` で指定された合法保有のローカル録画を使って Windows の smoke gate を実行します。最初のデコードフレーム、seek、一時停止と再開、字幕テクスチャ合成、リサイズ、capture、時間内の終了を検証します。私有の放送素材はリポジトリへ追加せず、公開回帰テストには合成データとプロトコルテストを使います。
