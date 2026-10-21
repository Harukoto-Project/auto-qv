# AutoQV

VRChat の QvPen などのペンワールドギミックでの筆記を自動化する、Windows 向けの CLI ツールです。

文字のストロークデータを元に、相対マウス移動で視点を動かしてクロスヘアを文字の軌跡通りになぞらせ、マウスボタンの押下/解放でペンの上げ下げを制御して書きます。

> **開発中です。** `write`(実際にマウスを動かして書く)と `preview`(入力を送らずに経路を確認する)が使えます。ストロークデータの自動取得は未実装のため、現時点ではストロークデータの JSON ファイルを自分で用意する必要があります。

## 動作環境

- Windows 10 / 11
- VRChat(QvPen などのペンが使えるワールド)

## 使い方(exe から実行)

`autoqv.exe` をコマンドプロンプトまたは PowerShell から実行します。exe の場所で次のように入力してください。

```powershell
.\autoqv.exe --help
.\autoqv.exe write --help
.\autoqv.exe preview --help
```

### `write`: 書く

```powershell
.\autoqv.exe write "HELLO" --glyph-file glyphs.json
.\autoqv.exe write --file message.txt --glyph-file glyphs.json
```

1. コマンドを実行すると、配置結果のサマリーを表示して待機状態になります。この間、マウスやキーボードの入力は一切送りません
2. VRChat のウィンドウに切り替え、ペンを持って書きたい位置にクロスヘアを合わせます(書き始めの位置が文字の左上になります)
3. **`Alt+P`** で書き始めます
4. **`Alt+C`** で中断できます(待機中に押すと何も書かずに終了します)。Ctrl+C でも中断できます

中断・終了時は、押していたマウスボタンを必ず離します。ホットキーはウィンドウのフォーカスに関係なく動作します。

テキストは `<TEXT>` 引数か `--file <PATH>`(UTF-8)のどちらか一方で指定します。`--dry-run` を付けると入力を送らず、サマリーだけ表示して終了します。

`write` では、`preview` のオプションに加えて次のオプションを使えます。

| オプション | 内容 | 既定値 |
| ---------- | ---- | ------ |
| `--button <BUTTON>` | ペンを下ろすために押すマウスボタン(`left` / `right` / `middle`) | left |
| `--start-hotkey <KEY>` | 書き始めるホットキー | `Alt+P` |
| `--abort-hotkey <KEY>` | 中断するホットキー | `Alt+C` |
| `--dry-run` | 入力を送らず、サマリーだけ表示して終了します | なし |

ホットキーは `Alt+P` や `Ctrl+Shift+F8` のように指定します。修飾キーは `Ctrl` / `Alt` / `Shift` / `Win`、キーは英字・数字・`F1`〜`F24`・`Esc`・`Space`・`Enter`・`Tab` です。英字・数字・`Space`・`Enter`・`Tab` は修飾キーが必須です。開始と中断には別のキーを指定してください。

#### 未収録文字のログ

ストロークデータに無い文字はスキップし、画面に表示するとともに、ログファイルに追記します(`--dry-run` でも記録します)。ログは 1 行 1 件の JSON です。

```json
{"timestamp":"2026-10-22T00:00:00Z","char":"ん","codepoint":"U+3093","line":1,"column":2}
```

既定の保存先は `%APPDATA%\Harukoto Project\AutoQV\autoqv-missing-glyphs.log` です。

### `preview`: 経路の確認

指定したテキストを書いた場合の経路を計算し、サマリーだけを表示します。マウスやキーボードの入力は送りません。

```powershell
.\autoqv.exe preview "Hello" --glyph-file glyphs.json
```

表示される内容:

- 配置した文字数、ストローク数、全体の大きさ(em)
- ペンダウン回数、移動回数、描画距離・ペンアップ移動距離(マウスカウント)
- 推定所要時間
- ストロークデータに無かった文字(行・列つき。最大 20 件)

#### オプション

| オプション | 内容 | 既定値 |
| ---------- | ---- | ------ |
| `--config <PATH>` | 設定ファイル(後述) | 実行ファイルと同じ場所の `autoqv.toml`(あれば) |
| `--glyph-file <PATH>` | ストロークデータの JSON ファイル。複数指定でき、後のものが優先されます | なし(現時点では必須) |
| `--size <N>` | 文字の大きさの倍率 | 1.0 |
| `--char-spacing <N>` | 字間(文字サイズに対する比) | 0.15 |
| `--line-spacing <N>` | 行間(文字サイズに対する比) | 0.4 |
| `--line-width-em <EM>` | 1 行の最大幅(em)。超えると折り返します | 20.0 |
| `--no-kinsoku` | 行頭禁則処理(句読点・閉じ括弧を行頭に置かない)を無効にします | 有効 |
| `--counts-per-em <N>` | 1 em あたりのマウスカウント数(縦横共通) | 400 |
| `--counts-per-em-x <N>` / `--counts-per-em-y <N>` | 軸ごとのカウント数(`--counts-per-em` より優先) | 400 |
| `--step-delay-ms <MS>` | マウス移動 1 ステップごとの待機時間 | 2 |
| `--max-counts-per-step <N>` | 描画中の 1 ステップあたりの最大カウント数 | 8 |
| `--move-speed-multiplier <N>` | ペンを上げて移動するときのステップ量の倍率 | 3.0 |

数値は、サイズ・感度・速度・待機時間が正の値、字間・行間が 0 以上の値のみ指定できます。

### 設定ファイル(`autoqv.toml`)

感度・速度・レイアウトなどを TOML ファイルに保存できます。`autoqv.exe` と同じ場所に `autoqv.toml` を置くと自動で読み込まれます。別の場所のファイルは `--config <PATH>` で指定します(指定したファイルが無い場合はエラーになります)。

すべての項目は省略できます。省略した項目は既定値になります。優先順位は「コマンドラインオプション > 設定ファイル > 既定値」です。

```toml
[sensitivity]
counts_per_em_x = 400.0
counts_per_em_y = 400.0

[pen]
button = "left"
step_delay_ms = 2
max_counts_per_step = 8
move_speed_multiplier = 3.0

[layout]
size = 1.0
char_spacing = 0.15
line_spacing = 0.4
line_width_em = 20.0
kinsoku = true

[log]
missing_glyph_log = "autoqv-missing-glyphs.log"

[safety]
start_hotkey = "Alt+P"
abort_hotkey = "Alt+C"
```

- 知らないキーやセクション名はエラーになります(タイプミスで設定が無視されないようにするためです)
- `missing_glyph_log` の相対パスは `%APPDATA%\Harukoto Project\AutoQV` が基準です。絶対パスも指定できます

#### 感度の調整

`counts_per_em_x` / `counts_per_em_y` は、文字の 1 em(文字の高さ・幅の基準)を書くのに送るマウスカウント数です。VRChat のマウス感度や視点の距離によって適切な値が変わるため、短い文字で試しながら調整してください。縦横で比率が合わない場合は、軸ごとに別の値を設定します。

### ストロークデータ(`--glyph-file`)

現時点では、ストロークデータを自動では取得しません。次の形式の JSON ファイルを自分で用意して指定します(今後、英数字・記号・ひらがな・カタカナ・漢字のデータを自動取得する予定です)。

```json
{
  "version": 1,
  "glyphs": {
    "I": {
      "advance": 0.5,
      "strokes": [[[0.25, 0.1], [0.25, 0.9]]]
    },
    "L": {
      "advance": 0.6,
      "strokes": [[[0.2, 0.1], [0.2, 0.9], [0.5, 0.9]]]
    }
  }
}
```

- 座標は 0.0〜1.0 の正方形(原点は左上、y は下向き)で表します
- `strokes` は「ストロークの配列 → 点の配列 → `[x, y]`」です
- `advance` は次の文字までの送り幅で、0 より大きい値にします
- キーは 1 文字だけにします

## 今後の予定

- ストロークデータの自動取得とキャッシュ(英数字・記号は Hershey Roman Simplex、ひらがな・カタカナ・漢字は KanjiVG)
- `check`: テキスト中のストロークデータに無い文字の確認

## 注意

- VRChat が管理者権限で動作している場合は、AutoQV も管理者権限で起動する必要があります(通常権限の入力は無視されます)
- 書き始めの位置は、待機中に合わせたクロスヘアの位置です。書き始める前に、ペンの位置と書き込める範囲を確認してください
- 書いている間は、マウスを動かしたりウィンドウを切り替えたりしないでください。位置がずれたり、別のウィンドウに入力が送られたりします
- コンソールウィンドウを閉じて終了すると、ボタンが押されたままになる場合があります。終了は `Alt+C` か Ctrl+C を使ってください
- 使用するワールドや VRChat の利用規約・ルールに従ってください

## 開発者向け

### 前提

- Rust(edition 2024 に対応した安定版)
- Windows(`SendInput` と `RegisterHotKey` を使用するため、Windows 以外では動作しません)

### ビルドと確認

```powershell
cargo build --release
cargo fmt
cargo clippy --all-targets
cargo test
```

ビルドした exe は `target\release\autoqv.exe` に出力されます。開発中は `cargo run -- preview "Hello" --glyph-file glyphs.json` で直接実行することもできます。

### デバッグ方法

**経路をマウスを動かさずに確認する**

`preview` は入力を送らないため、レイアウトや経路計画の確認に使えます。折り返し・禁則・感度を変えたときの値の変化を、次のように比較できます。

```powershell
cargo run -- preview "Hello, World" --glyph-file glyphs.json --counts-per-em 300
cargo run -- preview "Hello, World" --glyph-file glyphs.json --line-width-em 5 --no-kinsoku
```

`glyphs.json` に無い文字は「未収録」として行・列つきで表示されるので、データ不足の確認にも使えます。

`write --dry-run` は `preview` と同じサマリーを表示しますが、未収録文字をログファイルに記録する点が異なります。ログ出力の確認に使えます。

**実機に入力を送らずに `write` の流れを確認する**

`write` はホットキーの待機状態に入った時点では入力を送りません。VRChat などに切り替えずに、待機中に `Alt+C` を押すと何も送らず終了するので、ホットキーの登録や待機の動作だけを確認できます。ホットキーが他のアプリと衝突する場合は `--start-hotkey` / `--abort-hotkey` で変更してください。

**設定ファイルの確認**

`--config` で指定した設定ファイルの内容は、`preview` の出力(推定時間・移動距離)に反映されます。エラーメッセージにはファイルのパスとキー名が含まれます。

**テストの実行**

```powershell
cargo test
cargo test executor
cargo test -- --nocapture
```

- モジュール名・テスト名の一部を指定すると、その範囲だけ実行できます(例: `layout` `planner` `hotkey` `executor`)
- `--nocapture` を付けると、テスト中の標準出力を表示します

**パニック時のバックトレース**

```powershell
$env:RUST_BACKTRACE = "1"
cargo test
```

**テストしやすい構造**

- `glyph` / `layout` / `planner` / `runner::timing` / `input::hotkey` は純粋なロジックで、入力デバイスなしでテストできます
- `runner::executor::execute` は `PenDriver` trait を受け取るため、実機の代わりにテスト用のドライバ(`executor.rs` のテスト内の `FakeDriver`)で、分割・中断・ボタン解放の動作を確認できます
- 入力を送らず集計だけする `DryRunDriver`(`input/dry_run.rs`)は `preview` で使っています

**マウスボタンが押されたままになった場合**

実機での動作確認中にプロセスを強制終了すると、ペンのボタンが押されたままになることがあります。マウスを 1 回クリックすると解除できます。通常の終了・中断・panic・Ctrl+C では自動的に解放されます(コンソールウィンドウを閉じた場合を除く)。

**ホットキーが登録できない場合**

`failed to register hotkey ... it may be in use by another application` と表示されたときは、他のアプリが同じキーの組み合わせを使っています。`--start-hotkey` / `--abort-hotkey` か設定ファイルの `[safety]` で、別のキーの組み合わせに変更してください。

### ディレクトリ構成

```
src/
├─ cli/         コマンドライン引数とサブコマンド
├─ config/      設定ファイルの読み込みと検証
├─ glyph/       ストロークデータのモデルと読み込み
├─ layout/      テキスト → 配置済みストローク(折り返し・禁則)
├─ missing_log/ 未収録文字のログ出力
├─ planner/     配置済みストローク → 操作列(PenDown / PenUp / Move)
├─ input/       入力デバイス(SendInput・ホットキー・dry-run)
└─ runner/      操作列の実行(ステップ分割・待機・中断)
```

## ライセンスとデータの出典

ストロークデータは本リポジトリには含まれません。今後、次のデータを実行時に取得して利用する予定です。

- [Hershey Fonts](https://github.com/kamalmostafa/hershey-fonts)(Roman Simplex): 英数字・記号。A. V. Hershey 博士と James Hurt 氏による
- [KanjiVG](https://github.com/KanjiVG/kanjivg): ひらがな・カタカナ・漢字。Ulrich Apel 氏による。CC BY-SA 3.0
