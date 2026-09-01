[简体中文](timeline.md) · [繁體中文](timeline.zh-TW.md) · [日本語](timeline.ja.md) · [English](timeline.en.md)

> 本ページは翻訳です。簡体字中国語版のみを正式な情報源とします。

# タイムライン契約

タイムライン API は、デスクトップ UI に archive 全体をキャッシュせず、
上限のある archive ウィンドウをストリーミングします。

- `get_timeline_window` とフィルター付きの派生 API は完了済み archive をページングします。
- `get_timeline_recent_window_filtered` は完全な JSONL レコードを追尾します。
- `get_timeline_time_window` は範囲を限定した時間帯を返し、追記されたレコード上で
  バイトカーソルを進めます。

最終 JSONL 行が未完了の場合、読み取り側は後続の追記で完成するまで無視します。
タイムラインレコードはプロジェクト時間のミリ秒フィールドを使用します。preview の
media clock は明示的にマッピングし、曖昧な時間値として漏らしてはなりません。
Archive 形式と schema の規則は [`archive.md`](archive.md) にあります。
