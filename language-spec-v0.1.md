# 言語仕様 v0.1(ドラフト)

状態: 設計記録(ai-native-language-design.md)の決定を仕様の形に落としたもの。実装未着手。
言語名: 未定(仮称 `lang`)。
TBD と書かれた箇所は、フェーズ2のベンチマークで決める。

---

## 1. 概要

- 純粋・正格評価・静的型付きの関数型言語
- 正本は型付き ANF(A正規形)。v0.1 では ANF をそのまま表層構文とする
- 人間が読むのは仕様(契約・例・自然言語の説明)のみ。実装は AI が書き、機械が検証する
- 目標: 専門家が手で最適化した C と同等の性能を、検証付きで常時、少ないトークンで

### 1.1 設計原則(優先順)

1. 性能(専門家の C と同等、性能契約で検証)
2. 正しさ(曖昧さゼロ、未定義動作ゼロ)
3. 局所性(関数単体で振る舞いが確定)
4. トークン効率(読む+書く+やり直しの総量)
5. 実装層の可読性は要求しない

### 1.2 存在しないもの

例外、null、継承、可変の参照、共有可変状態、逃げるクロージャ、ユーザー定義マクロ、高カインド型、遅延評価、暗黙の型変換、未定義動作。

---

## 2. 字句

- ソースは UTF-8。改行は LF
- 識別子: `[a-z_][a-z0-9_]*`(関数・変数)、`[A-Z][A-Za-z0-9]*`(型・タグ・プール)
- ANF の一時名: `%` + 識別子または数字(`%1`, `%q`)
- 整数リテラル: `123`, `-5`, `0xFF`, `1_000`
- 浮動小数リテラル: `1.5`, `2.0e-3`
- 文字列リテラル: `"..."`(エスケープは `\n \t \\ \" \u{XXXX}`)
- 真偽: `true`, `false`
- コメント: `#` から行末まで。コメントは IR の意味に含まれず、ハッシュにも含まれない
- 予約語: `fn pub type pool cap requires ensures example let ret err match if else with par scope spawn perf schedule layout use import program where`

### 2.1 正規形

書き方は1通り。処理系の `lang fmt` が正規形に変換し、正規形でないソースは `lang check` が警告する。

- インデントは半角スペース2つ
- 1行に1命令
- 演算子は存在しない。全て前置の命令形(`add a b`)
- 命令の引数の間は半角スペース1つ

---

## 3. 型

### 3.1 基本型

| 型 | 内容 |
|---|---|
| `Int8 Int16 Int32 Int64` | 符号付き固定幅整数 |
| `UInt8 UInt16 UInt32 UInt64` | 符号なし固定幅整数 |
| `Float32 Float64` | IEEE 754 |
| `Bool` | `true` / `false` |
| `Char` | Unicode スカラ値 |
| `Str` | UTF-8 バイト列(不変) |
| `Unit` | 値 `()` のみ |

- デフォルトの整数型は `Int64`。リテラルは文脈から型が決まり、決まらなければ `Int64`
- 整数のオーバーフローは未定義ではない。静的に証明できなければ実行時に停止する(§9)
- 整数の除算: `div` は切り捨て、`rem` は剰余。0 による除算は型で禁止される(§3.5)
- 浮動小数は IEEE 754 に従う。NaN は値として存在するが、`==` で NaN は等しくない

### 3.2 SIMD 型(TBD、フェーズ6)

`Float32x8`, `Int32x4` など、幅つきのベクトル型。演算は要素ごと。

### 3.3 複合型

```
# タプル
(Int32, Str)

# レコード
{x: Float64, y: Float64}

# 固定長配列(長さはコンパイル時定数)
[Float32; 1024]

# 可変長配列(領域に属する。§7)
[Float32]

# オプション
?Int32            # None | Some(Int32)

# 結果(§8)
Int32 ! {NotFound, Invalid{line: Int64}}
```

### 3.4 代数的データ型

```
type Shape =
  | Circle {r: Float64}
  | Rect {w: Float64, h: Float64}
  | Dot
```

- タグは `Shape.Circle` のように型名で修飾する。文脈から明らかなら省略可
- 再帰型は可(`type Tree = | Leaf | Node {l: Tree, v: Int32, r: Tree}`)。再帰型の値は領域またはプールに属する
- 構造的等価・順序・ハッシュ・表示・シリアライズは自動導出される。ただし `Float` を含む型は順序を導出しない

### 3.5 篩型

```
Int32 where x != 0
Int64 where 0 <= x && x < 100
[Float32] where len(x) > 0
{a: Int32, b: Int32} where x.a <= x.b
```

- `x` は篩条件の中でその値自身を指す束縛変数
- 篩条件に書けるのは**決定可能な断片**のみ:
  - 線形整数算術(`+ - * (定数倍) < <= == !=`)
  - 真偽演算(`&& || !`)
  - `len(配列)`, `len(文字列)`
  - 他の純粋関数の呼び出しは**未解釈関数**として扱う(等しい引数なら等しい結果、以上のことは仮定しない)
- 篩型 `T where p` は `T` の部分型。`T where p` の値は `T` が要求される場所に渡せる。逆は証明が必要
- 篩条件の検証は SMT ソルバ(Z3)で行う。証明できない場合の扱いは §9

### 3.6 ジェネリクス

```
fn first<T>(xs: [T] where len(x) > 0) -> T
```

- 型引数は推論される。推論できない場合は呼び出し側で `first<Int32>` と明示
- 全て単相化(monomorphization)される。実行時の型情報は存在しない
- 型引数に対する能力制約(§3.7)を付けられる: `fn max<T: Ord>(a: T, b: T) -> T`

### 3.7 能力(abilities)

組み込みの能力: `Eq`, `Ord`, `Hash`, `Show`, `Serialize`。レコード・タプル・ADT は構成要素が全てその能力を持てば自動で持つ。

ユーザー定義の能力(TBD、最小限):

```
ability Area {
  fn area(self) -> Float64
}
impl Area for Shape {
  fn area(self) -> Float64 { ... }
}
```

高カインド型は存在しない(`Functor` のような抽象化は書けない)。

---

## 4. 関数と ANF

### 4.1 関数定義

```
fn safe_div(a: Int32, b: Int32 where x != 0) -> Int32
  ensures r == div a b
  example safe_div 10 2 == 5
  example safe_div -7 2 == -3
{
  %q = div a b
  ret %q
}
```

順序は固定: 名前と引数 → 戻り型 → `requires` → `ensures` → `perf` → `example` → 本体。
引数の型は必須。戻り型は必須。

### 4.2 本体

本体は命令の列。各命令は次のいずれか:

```
%名前 = 命令 引数...          # 束縛(1行1命令)
let 名前 = 命令 引数...       # 束縛(人間が名前を付けたい場合。%名前と同じ)
ret 値                        # 戻り
ret err Tag {...}             # エラーで戻る(§8)
if 値 { ... } else { ... }    # 分岐。両方の腕が同じ型の値で ret する
match 値 { ... }              # パターンマッチ(§4.4)
```

- 束縛は不変。再代入は存在しない
- 本体内の型は全て推論される。注釈はオプション: `%q : Int32 = div a b`
- 全ての実行経路は `ret` で終わる

### 4.3 組み込み命令

```
# 整数・浮動小数
add sub mul div rem neg
# 比較(結果は Bool)
eq ne lt le gt ge
# 真偽
and or not
# 変換(明示のみ)
cast_i32 cast_i64 cast_f64 ...     # 失敗しうる変換は ?T か T ! {Overflow} を返す
# 配列
len index slice push concat
# レコード・タプル・ADT
make Tag {...}   get 値 .field   
# 関数
call 関数名 引数...
```

`index xs i` の `i` は `Int64 where 0 <= x && x < len(xs)` を要求する。要求を満たさない添字はコンパイルエラー。

### 4.4 パターンマッチ

```
match s {
  Circle {r} -> { %a = mul r r ; ret %a }
  Rect {w, h} -> { %a = mul w h ; ret %a }
  Dot -> { ret 0.0 }
}
```

- 網羅性は必須。漏れはコンパイルエラー
- ガード: `Rect {w, h} where w == h -> ...`
- 篩型と連動: `match` の腕の中では、その腕の条件が篩条件として使える

### 4.5 関数値(逃げないクロージャ)

```
%ys = map xs with |x| { %y = mul x 2 ; ret %y }
%s  = fold xs 0 with |acc, x| { %t = add acc x ; ret %t }
```

- `with |引数| { 本体 }` のブロックは、引数として渡す先でのみ使える
- ブロックは囲む関数のローカル変数を参照できる
- ブロックを値として束縛、保存、返却、スレッドに渡すことはできない(コンパイルエラー)
- ブロックはコンパイル時に展開され、実行時の関数値は存在しない
- ユーザー定義関数もブロックを受け取れる: `fn each<T, F>(xs: [T], f: F)`。`F` は関数値型で、呼び出し `call f x` のみ可

### 4.6 再帰と停止性

- 再帰は可
- 全ての関数はデフォルトで停止性が要求される。処理系は引数のうち構造的に減少するものを自動で探す。見つからなければ `decreases 式` で指示する
- 終わらない関数は `fn serve(...) -> Never` と宣言する。`Never` を返す関数は停止性の対象外(TBD: 終わらない関数の扱いは §11 の並行と合わせて詰める)

---

## 5. 契約と例

### 5.1 契約

```
requires 条件      # 事前条件。呼び出し側の責任
ensures 条件       # 事後条件。戻り値は r で参照
```

- 条件に書けるものは篩条件と同じ断片(§3.5)
- `requires` は引数の篩型と等価。どちらで書いてもよいが、正規形では引数の篩型に寄せる
- 契約違反の実行時の扱いは §9

### 5.2 例

```
example safe_div 10 2 == 5
example safe_div 1 0 fails         # 型エラーになることを示す例(コンパイル時に確認)
example parse "abc" == err Invalid {line: 1}
```

- `example` は仕様・ドキュメント・テストを兼ねる
- `lang check` は全ての例を実行し、結果を報告する
- 処理系は境界の例(0、負数、最大値、空、重複)を自動生成して提案できる(`lang suggest-examples`)
- 例は人間が承認する仕様の一部。例の変更は関数のハッシュを変える

### 5.3 自然言語の説明

```
fn safe_div(...)
  doc "a を b で割った商を返す。切り捨て。b は 0 であってはならない。"
```

- `doc` は処理系が契約と例から生成し、人間が承認する。手書きも可
- `doc` はハッシュに含まれない(意味を持たないため)

---

## 6. 効果と権限(ケイパビリティ)

全ての副作用は、権限を表す値(ケイパビリティ)を引数として受け取った関数だけが起こせる。

### 6.1 ケイパビリティ型

```
cap Stdin
cap Stdout
cap FsRead  {root: Str}             # root 以下の読み取り
cap FsWrite {root: Str}
cap Net     {host: Str, scheme: Str}
cap Clock
cap Random
cap Spawn                            # タスクの起動(§11)
```

- ケイパビリティは篩型を持てる: `Net where x.host == "api.example.com"`
- ケイパビリティは偽造できない。`program`(§6.2)から渡されたものを引数で受け渡すしかない
- ケイパビリティを**分割**する関数は組み込みで提供される: `fs_narrow fs "sub/"` は `FsRead {root: "sub/"}` を返す。広げることはできない

### 6.2 プログラムの入口

```
program calc
  requires [Stdin, Stdout]
  doc "対話式電卓"
{
  fn main(stdin: Stdin, stdout: Stdout) -> Unit ! {IoError} { ... }
}
```

- `program` の `requires` が、このプログラムが要求する権限の全てである。人間はこの一覧を見て承認する
- 要求していない権限は、どのコードからも使えない

### 6.3 外部関数(FFI)

```
extern fn c_sqrt(x: Float64) -> Float64
  from "libm" symbol "sqrt"
  requires_cap Unsafe
```

- 全ての `extern fn` は `Unsafe` ケイパビリティを要求する
- `extern fn` は常に「未検証」(§9)。境界で引数と戻り値の型検査を処理系が挿入する
- 渡せる型は C 表現を持つもののみ

---

## 7. メモリ: 領域とプール

### 7.1 領域(暗黙)

- 全ての値は、それを作った関数の呼び出し(スコープ)に属する領域に置かれる
- 領域はスコープの終了時に一括解放される
- 値を返すと、呼び出し元の領域に**構築される**(コピーではなく、その場で構築。Goose の in-place construction と同じ保証)
- 入れ子のデータ(配列のレコードの配列…)は、ポインタを含まない1つの連続したブロックに置かれる
- 領域の推論は処理系が行い、注釈は不要

### 7.2 可変長配列

```
%xs = make_array<Int32>            # 空
%ys = push xs 1                     # 純粋に見えるが、xs を他が参照していなければその場で伸びる
```

- `[T]` は属する領域の中で成長する。成長で既存の要素は動かない
- 再帰呼び出しをまたいで成長する配列は持てない(コンパイルエラー。プールに渡す)

### 7.3 プール(明示)

```
pool Nodes : Node                   # Node 型の値を入れるプール

fn build(p: Nodes, ...) -> ... {
  %n = alloc_in p (make Node {...})   # p の中に置く
}
```

- スコープに寿命が収まらない値、長生きする値、再利用される値はプールに置く
- プールは値として関数に渡す。プールへの参照は、プールより長生きできない(処理系が検査)
- プールの要素への参照 `Ref<Node>` は、プールの中では安定している
- プール内の解放は `free_in p ref`。解放済みの参照からの読み出しは「別の正しい型の値」を読むことがあるが、メモリ破壊にはならない(Goose と同じ)

### 7.4 参照カウント(例外)

- 領域にもプールにも置けないと処理系が判断した値だけが、参照カウント管理になる
- `perf no_rc` が付いた関数の内部では、参照カウント操作が一切発生しないことを処理系が保証する。発生する場合はコンパイルエラー
- `perf no_rc` はデフォルトで**有効**(TBD: フェーズ2で測って決める)

### 7.5 配置の指定(TBD、フェーズ6)

```
layout Particle = soa                # 配列の構造体
layout Vertex = packed align 16
```

---

## 8. エラー

### 8.1 Result 型

```
fn parse(s: Str) -> Expr ! {Empty, Invalid {pos: Int64}}
```

- `T ! {Tags}` は、成功で `T`、失敗でタグのいずれかを返す
- `ret err Tag {...}` で失敗を返す
- 呼び出し側は `match` で全てのタグを処理するか、`try` で伝播する

### 8.2 伝播

```
%e = try call parse s      # parse が失敗したら、同じタグで自分も失敗する
```

- `try` を使う関数の戻り型のタグ集合は、伝播する全てのタグを含んでいなければならない
- 処理系はタグ集合を**自動で合算**して提案する(`lang infer-errors`)。書き忘れはコンパイルエラー

### 8.3 バグと契約違反

- 範囲外アクセス、0除算、オーバーフロー、契約違反は**エラー値ではない**。可能な限り型で禁止される(§3.5)
- 型で禁止できず実行時に残ったチェックが失敗した場合、プログラムは即座に停止する。停止時の情報は構造化データ(§12)で出力される。黙って続行することはない

---

## 9. 検証と信頼レベル

### 9.1 検証の対象

関数ごとに、次が検証される:
- 引数の篩型が本体で守られること
- `ensures` が全ての実行経路で成り立つこと
- `index` などの組み込み命令の要求が満たされること
- 停止性
- `perf` 契約(§10)

### 9.2 信頼レベル

各関数に、検証結果として次のいずれかが付く:

| レベル | 意味 |
|---|---|
| `proven` | 全ての入力について、契約が成り立つことが SMT で証明された |
| `tested` | 証明はできなかったが、`example` と自動生成した入力(デフォルト 10,000 件)で契約違反が見つからなかった。実行時チェックが残る |
| `unverified` | 検証が時間切れまたは不能。実行時チェックのみ。`extern fn` は常にこれ |

- `lang check` は関数ごとのレベルと、`tested` / `unverified` になった理由(証明できなかった条件と**反例**、または時間切れ)を出力する
- `program` には `requires_level proven` のように最低レベルを要求できる。満たさなければビルドできない
- 信頼レベルは関数の IR のハッシュに紐づいてキャッシュされる。IR が変わらなければ再検証しない

### 9.3 反例

証明に失敗したとき、処理系は反例(契約を破る具体的な入力)を返す。反例はそのまま `example ... fails` として仕様に追加できる。

---

## 10. 性能契約

```
fn dot(a: [Float32], b: [Float32] where len(x) == len(a)) -> Float32
  perf no_alloc
  perf checkfree
  perf vectorize
```

| 契約 | 意味 |
|---|---|
| `no_alloc` | 関数内で領域の成長もプール割り当ても起きない |
| `no_rc` | 参照カウント操作が残らない |
| `checkfree` | 実行時の境界チェック・オーバーフローチェックが残らない |
| `vectorize` | 主ループがベクトル化される |
| `complexity O(式)` | 計算量の上限(TBD、フェーズ6) |

- 契約を満たせない場合はコンパイルエラー。エラーには「どの命令の、どのチェックが残ったか」が含まれる
- デフォルトで有効な契約: `no_rc`(TBD)。他はデフォルト無効(TBD: フェーズ2で測って決める)
- `lang perf-report` は、契約の有無にかかわらず、残ったチェック・割り当て・ベクトル化の可否を関数ごとに構造化データで出力する

### 10.1 スケジュール層(TBD、フェーズ6)

```
schedule for dot {
  vectorize loop0 width 8
  unroll loop0 by 4
}
schedule for life_step {
  parallel loop0
  tile loop0 loop1 by 64 64
}
```

- スケジュールはコア層の意味を変えられない。どんなスケジュールでも結果は同じ
- スケジュールは関数のハッシュに含まれない(意味を持たないため)。ただし性能契約の検証対象にはなる

---

## 11. 並行と並列

### 11.1 並列(データ並列)

```
%ys = par_map xs with |x| { ... }
%s  = par_fold xs 0 with |a, b| { ... }     # 結合律を満たす演算のみ。ensures で示す
```

- 結果は逐次版(`map`, `fold`)と必ず同一
- スレッド数は処理系が決める
- `par_fold` の演算は結合律を要求する。`ensures` で `f (f a b) c == f a (f b c)` を示す(証明できなければ `tested`)

### 11.2 並行(構造化)

```
scope sp {                         # sp: Spawn ケイパビリティ
  %t1 = spawn sp with || { ... }
  %t2 = spawn sp with || { ... }
  %r1 = wait t1
  %r2 = wait t2
}                                   # scope を抜けるとき、全ての子は終わっている
```

- `spawn` には `Spawn` ケイパビリティが必要
- 起動したタスクは `scope` の終了までに必ず終わる。`scope` の外にタスクは存在しない
- タスク間に共有状態はない。やり取りはチャネル(`chan<T>`)で、送れるのは**平坦な値**(参照を含まない値)のみ
- 呼び出しの色分けはない。`wait` は普通の関数呼び出しに見え、待ち合わせは処理系が行う

### 11.3 決定的シミュレーション

- `lang test --seed N` は、タスクの実行順序を種 N で固定して実行する
- 失敗した種を指定すれば、同じ順序を再現できる

---

## 12. 処理系のインターフェース

### 12.1 コマンド

```
lang run file            # インタプリタで即実行
lang check file          # 型検査・検証・例の実行。結果を構造化データで出力
lang fmt file            # 正規形に整形
lang build file          # C 出力 → ネイティブ実行ファイル
lang describe [項目]     # 言語仕様・標準ライブラリを、AI のプロンプト向けに出力
lang query file 質問     # 型、依存、検証状態などへの問い合わせ(§12.3)
lang perf-report file
lang suggest-examples file fn
lang infer-errors file fn
lang test --seed N
```

### 12.2 構造化エラー

全てのエラー・警告・検証結果は JSON で出力される(`--human` で文章に変換)。

```json
{
  "kind": "refinement_violation",
  "location": {"file": "calc.lang", "fn": "safe_div", "line": 12, "col": 8},
  "expected": "Int32 where x != 0",
  "actual": "Int32",
  "counterexample": {"b": 0},
  "suggestion": "引数 b に篩型 `where x != 0` を付けるか、呼び出し前に b を検査してください",
  "related": [{"fn": "main", "line": 30}]
}
```

必須フィールド: `kind`, `location`, `expected`, `actual`。任意: `counterexample`, `suggestion`, `related`。

### 12.3 クエリ(TBD、フェーズ7)

```
lang query f.lang "type-of safe_div"
lang query f.lang "depends-on parse"
lang query f.lang "proven-facts dot"
lang query f.lang "impact-of-change safe_div"
```

### 12.4 構文木の編集(TBD、フェーズ7)

```
lang edit f.lang replace-body safe_div < new_body.lang
lang edit f.lang add-param safe_div "c: Int32"
```

### 12.5 typed hole

```
%r = hole : Int32      # 未実装。型だけ宣言
```

- `hole` を含む関数は `unverified` 扱いで、実行時に `hole` に到達すると停止する
- `lang check` は `hole` の一覧と、その場所で利用可能な変数と型を出力する

---

## 13. モジュール

- ファイル = モジュール。ディレクトリがそのまま名前空間
- `pub fn` だけが外から見える。`pub` には承認済みの仕様(契約と例)が必須
- `use dir.module` で参照。循環依存はコンパイルエラー
- 外部ライブラリ:

```
import json from "owner/repo" hash "sha256:abcd..."
```

- 外部ライブラリは `unverified` 扱い。要求する権限は `lang check` が一覧表示する
- 同じライブラリの別ハッシュは共存できる

---

## 14. 同一性とハッシュ

- 関数の同一性は、その IR のコア層(型・篩条件・領域・権限・本体)の正規形から計算した SHA-256
- 名前、コメント、`doc`、スケジュール、配置注釈はハッシュに含まれない
- 承認、信頼レベル、検証結果、性能レポートは全てハッシュに紐づく。IR が変われば全て失効する

---

## 15. コンパイルパイプライン

```
ソース(ANF テキスト)
  → 字句・構文解析
  → 正規化(fmt と同じ)
  → 型検査・篩型の制約生成
  → 領域推論
  → 権限検査
  → SMT による検証(信頼レベル決定)
  → 例の実行
  → [run] インタプリタで実行
  → [build] 性能契約の検査 → スケジュール適用 → C 出力 → 翻訳検証 → clang/gcc
```

- 全ての段階は決定的。同じ入力からはビット単位で同じ出力
- C 出力は人間が読む前提ではない
- 翻訳検証(TBD、フェーズ5): C 出力と IR の等価性を小さなチェッカーで確認する

---

## 16. 文法(EBNF、v0.1 の範囲)

```
program     = { item } ;
item        = fn_def | type_def | pool_def | cap_def | program_def | use | import ;

fn_def      = [ "pub" ] "fn" ident [ generics ] "(" [ params ] ")" "->" type
              { "requires" expr } { "ensures" expr } { "perf" perf }
              { "example" example } [ "doc" string ]
              block ;
params      = param { "," param } ;
param       = ident ":" type ;
type        = base_type [ "where" expr ] ;
base_type   = ident [ "<" type { "," type } ">" ]
            | "(" type { "," type } ")"
            | "{" field { "," field } "}"
            | "[" type [ ";" int ] "]"
            | "?" type
            | type "!" "{" tag { "," tag } "}" ;
field       = ident ":" type ;

block       = "{" { stmt } "}" ;
stmt        = bind | ret | if | match ;
bind        = ( "%" name | "let" ident ) [ ":" type ] "=" instr ;
instr       = ident { operand } [ "with" lambda ] ;
operand     = "%" name | ident | literal | ident "." ident ;
lambda      = "|" [ ident { "," ident } ] "|" block ;
ret         = "ret" operand | "ret" "err" tag [ record ] ;
if          = "if" operand block "else" block ;
match       = "match" operand "{" { arm } "}" ;
arm         = pattern [ "where" expr ] "->" block ;
pattern     = tag [ "{" ident { "," ident } "}" ] | "_" | literal ;

type_def    = "type" Ident [ generics ] "=" { "|" tag [ "{" field { "," field } "}" ] } ;
pool_def    = "pool" Ident ":" type ;
cap_def     = "cap" Ident [ "{" field { "," field } "}" ] ;
program_def = "program" ident "requires" "[" { Ident } "]" [ "doc" string ] "{" { item } "}" ;

expr        = (* 篩条件・契約用の式。§3.5 の断片。前置記法ではなく中置記法を許す *) ;
```

---

## 17. 未決事項(TBD)一覧

| 項目 | 決める時期 |
|---|---|
| 言語名 | いつでも |
| 表層構文に糖衣を足すか(S式 / 案B / Python 風) | フェーズ2 |
| `perf` のデフォルト(`no_rc` 以外) | フェーズ2 |
| 停止性の扱いと `Never` | フェーズ3 |
| ユーザー定義能力の構文 | フェーズ3 |
| SIMD 型、配置指定、スケジュール層の構文 | フェーズ6 |
| 翻訳検証の方式 | フェーズ5 |
| クエリと構文木編集のコマンド体系 | フェーズ7 |

---

## 付録 A. 最初の題材(ステップ0〜5 の電卓)

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

ステップ3以降:

```
fn safe_div(a: Int32, b: Int32 where x != 0) -> Int32
  ensures r == div a b
  example safe_div 10 2 == 5
  example safe_div 1 0 fails
{
  %q = div a b
  ret %q
}
```
