# oyomi

[![CI](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml/badge.svg)](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml)

お読み。Office 文書（xlsx / docx / pptx）を決定論的なテキストに落とし、`git diff` で何が変わったのか読めるようにする。

English: [README.md](README.md)

## 何が問題か

git は Office 文書を不透明なかたまりとして扱い、`Binary files ... differ` としか言わない。仕様書を Excel や Word で持っていると、MR で変更をレビューできない。

既存の回避策はどれも途中で止まっている。

`soffice --headless --convert-to csv` は**先頭シートしか変換しない**。3 シートのうち 3 枚目に中身がある帳票を通したところ、CSV は 4 行になり、620 セルのシートは消えた。ウォーム状態でも 1 ファイル 1.5 秒、初回は 10 秒を超える。

openpyxl は全シート読めるが、遅いうえに整形は呼び出し側まかせになる。

どちらも、レビューで問われること、つまり**その出力は差分として読めるのか**を見ていない。

## インストール

```console
$ cargo install --path .
```

## 使い方

```console
$ oyomi catalog.xlsx
Cover!A1	Document
Cover!F1	Job Catalog
Items!B62	JOB900
...

$ oyomi --list catalog.xlsx
Cover	4x6	8 cells
History	3x5	9 cells
Items	124x5	620 cells

$ oyomi --sheet Items catalog.xlsx
```

| オプション | 動き |
|---|---|
| （なし） | 1 セル 1 行。アドレスがキーになる（`Sheet!B62<TAB>値`）。場所を特定したいとき |
| `--row` | 1 行 1 行。キーに位置を入れない。差分に使うのはこちら |
| `--list` | シート名と寸法と非空セル数だけ |
| `--sheet NAME` | そのシートだけ |

docx は段落と表のセルを 1 行ずつ、pptx はスライド番号つきで段落を 1 行ずつ出す。

## git textconv として使う

```console
$ cat .gitattributes
*.xlsx diff=oyomi
*.xlsm diff=oyomi
*.docx diff=oyomi
*.pptx diff=oyomi

$ git config diff.oyomi.textconv "oyomi --row"
$ git config diff.oyomi.cachetextconv true
```

`git diff --stat` だけはバイト数のまま出る。git が stat に textconv を通さないため。

## キーに位置を入れない理由

ここがこのツールの中身のほとんどを占める。

最初の版は `Sheet!B62<TAB>値` だけを出していた。1 セルの編集はきれいに読める。ところが**行を挿入すると以降の行番号が全部ずれる**ので、620 セルのシートに 3 行足しただけで 314 行の差分になった。

キーから行番号を外すと、同じ変更が 6 行になる。

```diff
+History	A=2	C=1.1	E=added three jobs
+Items	A=group-09	B=JOB900	C=batch	D=new job 900	E=yes
+Items	A=group-09	B=JOB901	C=batch	D=new job 901	E=yes
+Items	A=group-09	B=JOB902	C=batch	D=new job 902	E=yes
```

docx も同じで、段落番号をキーにすると、60 段落の文書に 3 段落を挿入した差分が 93 行になる。番号を外すと 7 行になり、版数が上がったことと節が増えたことがそのまま読める。

位置から作ったキーは、1 つ挿入されただけで以降が全部変わる。diff が行を対応づけられなくなるので、キーは内容から作るか、持たせない。

## 計測

実文書ではなく生成したサンプルでの数字。3 シート 637 セルの xlsx と、63 段落の docx を使った。

| | oyomi | 比較対象 |
|---|---|---|
| xlsx 1 件（3 シート 637 セル） | 0.006s | soffice 1.54s（ウォーム）、かつ 2 シート欠落 |
| xlsx 20 件 | 0.107s | openpyxl 0.366s |
| 3 行挿入の差分サイズ | 6 行（`--row`） | 314 行（セルキー） |
| 3 段落挿入の差分サイズ | 7 行（`--row`） | 93 行（段落番号） |

## テスト

```console
$ cargo test
$ cargo llvm-cov --summary-only
```

テスト用の文書は全て `tests/common/mod.rs` で生成している。xlsx・docx・pptx のパッケージ、
2 枚目のシートが宣言だけされていて実体のないブック、UTF-8 として不正なパート、
どの読み手も開けないエントリを持つ手組みの zip まで作る。リポジトリ自身はバイナリを持たない。

行カバレッジは 99.6%。到達していない 1 箇所は、zip の中央ディレクトリが自分のエントリと
食い違っている場合に備えた分岐。

## 未対応

画像だけで出来た文書は空になる。`<media: N files>` のような行を出さないと、スクリーンショットを差し替えても差分が空になる。

数式を見ていない。calamine が返すのはキャッシュ済みの計算結果なので、式を書き換えても値が同じだと差分に出ない。

xlsx の図形内テキストとセルのコメントを拾っていない。

docx の変更履歴は `w:del` を `<del>` で囲むところまで。`w:ins` は素通ししている。

ノートはパッケージの関係定義ではなくパート名の番号でスライドに対応づけている。ノートの番号がスライドと独立に振られた資料では、対応先を取り違える。

## ライセンス

MIT
