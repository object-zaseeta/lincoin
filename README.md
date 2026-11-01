# Lincoin(仮称)— AI が書き、機械が検証する言語

専門家が手で最適化した C と同等のコードを AI が少ないトークンで常時生成でき、人間は仕様の承認だけで出荷できるプログラミング言語を作るプロジェクトです。
言語名は未定で、リポジトリ名の Lincoin とコマンド名の `lang` は仮です。

> A programming language written by AI and verified by machines: C-class performance, low token cost, and humans ship by approving specs and examples instead of reading code. Design documents are in Japanese; commit messages are in English.

## なぜ作るか

AI がコードを書くほどコードの量は人間が読める範囲を超え、検証が人間のボトルネックになります。
この言語はそのボトルネックを言語側で引き受けます。
人間が読むのはコードではなく仕様と具体例で、正しさの根拠は動作確認ではなく機械検証(証明済み / テスト済み / 未検証の 3 段階)です。

| | 既存言語に AI が書く | この言語 |
|---|---|---|
| 人間が読むもの | コード(読めない量) | 仕様と具体例 |
| 正しさの根拠 | 動作確認、AI への信頼 | 機械検証 |
| 何に触れるか | コードを読まないと分からない | 権限の一覧として見える |
| 承認後の改変 | 気づけない | ハッシュが変わり承認が自動失効 |
| 速さ | AI の腕次第 | 性能契約で保証 |

詳しくは `vision.md` を読んでください。

## いまできること(フェーズ 1 / ステップ 0)

Rust 製のインタプリタ `lang run` が、ANF(A 正規形。式をすべて一時変数への代入に分解した形)で書いた Int32 の固定式を実行します。
オーバーフローと 0 除算では、AI が読むための構造化エラー(JSON)を返して止まります。

```
$ cargo run -q -- run examples/calc.lang
85

$ cargo run -q -- run examples/div_zero.lang
{"kind":"division_by_zero","location":{"file":"examples/div_zero.lang","fn":"main","line":8,"col":10},"expected":"Int32 where x != 0","actual":"div 5 0","suggestion":"..."}
```

`examples/calc.lang` の中身はこうです。

```
program calc
  requires []
  doc "固定式の電卓"
{
  fn main() -> Int32
    example main == 85
  {
    %1 = add 10 20
    %2 = mul %1 3
    %3 = sub %2 5
    ret %3
  }
}
```

終了コードは 0 = 完走、1 = プログラムが止まった(オーバーフロー・0 除算・契約違反)、2 = コンパイルできない(構文・入出力)、64 = コマンドラインの誤りです。

## 試す

Rust のツールチェーン(cargo)が必要です。

```sh
cargo test              # ユニット 26 本 + CLI 6 本
cargo run -- run examples/calc.lang
```

## ロードマップ

電卓一本を「動く → 契約で守られる → 証明される」と育てたあと、ゲームで表現力を、マンデルブロ集合とライフゲームで速さを見せます。

| フェーズ | 内容 | 状態 |
|---|---|---|
| 0 | 設計原則と決定記録 | 完了 |
| 1 | 極小のコア(整数・関数・分岐・契約)のインタプリタ | ステップ 0 完了、ステップ 1〜3 が残り |
| 2 | 評価基盤(同じ課題を既存言語と比べて AI に解かせるベンチマーク) | 未着手 |
| 3 | 検証(契約を Z3 で証明、信頼レベルの提示) | 未着手 |
| 4 | 効果と権限(ケイパビリティ) | 未着手 |
| 5 | C 出力と翻訳検証、性能契約 | 未着手 |
| 6 | Halide 式スケジュール、SIMD、メモリ配置 | 未着手 |
| 7 | 常駐コンパイラ、IR 直接編集、証明運搬コード、Wasm | 未着手 |

ステップごとの題材と結果は `ai-native-language-design.md` の §8 にあります。

## 完成の定義

次の 3 つを全て満たしたとき v1.0 とします。数値は仮置きで、フェーズ 2 のベンチマーク基盤ができた時点で確定します。

1. **性能**: 性能契約を通ったコードが、公開最速級の C 実装と同等(±10%)の速度を出す
2. **常時・安価**: 複数社のフロンティア級モデルの全てで、性能契約を通る成功率が 95% 以上、総トークンが同じモデルに Python で解かせた場合以下
3. **承認だけで出荷**: 人間が仕様と具体例だけを見て承認したロジックが、手書きのテストなしで 30 日以上バグ修正なしに稼働する

## リポジトリの構成

| パス | 内容 |
|---|---|
| `vision.md` | なぜ作るか、完成の定義、反証条件 |
| `ai-native-language-design.md` | 設計判断の記録(決定・理由・却下案)、ロードマップ、各ステップの結果 |
| `language-spec-v0.1.md` | 言語仕様ドラフト |
| `docs/grill/` | 設計の詰めの記録(Grill Document) |
| `src/` | インタプリタ(`lexer` → `parser` → `interp`、エラーは `error`) |
| `examples/` | 各ステップの題材(`.lang`) |
| `tests/` | バイナリを実際に起動する E2E テスト |

## ライセンス

MIT。`LICENSE` を参照してください。
