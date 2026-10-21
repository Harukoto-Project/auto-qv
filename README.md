# AutoQV

VRChat の QvPen などのペンワールドギミックでの筆記を自動化する、Windows 向けの CLI ツールです。

文字のストロークデータを元に、相対マウス移動で視点を動かしてクロスヘアを文字の軌跡通りになぞらせ、マウスボタンの押下/解放でペンの上げ下げを制御して書きます。

> **開発中です。** 現時点で使えるのは `preview`(入力を一切送らずに経路を確認するコマンド)だけです。実際にマウスを動かして書く `write` コマンドと、ストロークデータの自動取得は未実装です。

## 動作環境

- Windows 10 / 11
- VRChat(QvPen などのペンが使えるワールド)

## 使い方(exe から実行)

`autoqv.exe` をコマンドプロンプトまたは PowerShell から実行します。exe の場所で次のように入力してください。

```powershell
.\autoqv.exe --help
.\autoqv.exe preview --help
```

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

- `write`: 待機状態になり、`Alt+P` で書き始め、`Alt+C` で中断する(ホットキーは設定可能)
- `check`: テキスト中のストロークデータに無い文字の確認
- ストロークデータの自動取得とキャッシュ(英数字・記号は Hershey Roman Simplex、ひらがな・カタカナ・漢字は KanjiVG)
- 設定ファイル(`autoqv.toml`)

## 注意

- 実際に書き込む機能が実装された後も、VRChat が管理者権限で動作している場合は、AutoQV も管理者権限で起動する必要があります(通常権限の入力は無視されます)
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

`failed to register hotkey ... it may be in use by another application` と表示されたときは、他のアプリが同じキーの組み合わせを使っています。設定でホットキーを変更してください(`write` の実装後に対応)。

### ディレクトリ構成

```
src/
├─ cli/       コマンドライン引数とサブコマンド
├─ glyph/     ストロークデータのモデルと読み込み
├─ layout/    テキスト → 配置済みストローク(折り返し・禁則)
├─ planner/   配置済みストローク → 操作列(PenDown / PenUp / Move)
├─ input/     入力デバイス(SendInput・ホットキー・dry-run)
└─ runner/    操作列の実行(ステップ分割・待機・中断)
```

## ライセンスとデータの出典

ストロークデータは本リポジトリには含まれません。今後、次のデータを実行時に取得して利用する予定です。

- [Hershey Fonts](https://github.com/kamalmostafa/hershey-fonts)(Roman Simplex): 英数字・記号。A. V. Hershey 博士と James Hurt 氏による
- [KanjiVG](https://github.com/KanjiVG/kanjivg): ひらがな・カタカナ・漢字。Ulrich Apel 氏による。CC BY-SA 3.0
