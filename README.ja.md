# oyomi

[![CI](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml/badge.svg)](https://github.com/HaiTo/oyomi/actions/workflows/ci.yml)

お読み。Office 文書（xlsx / docx / pptx）を決定論的なテキストに落とし、`git diff` で何が変わったのか読めるようにする。

English: [README.md](README.md)

## 何が問題か

git は Office 文書を不透明なかたまりとして扱い、`Binary files ... differ` としか言わない。仕様書を Excel や Word で持っていると、MR で変更をレビューできない。

既存の回避策はどれも途中で止まっている。

`soffice --headless --convert-to csv` は**先頭シートしか変換しない**。3 シートのうち 3 枚目に中身がある帳票なら、CSV は 4 行になり、620 セルのシートは出てこない。ウォーム状態でも 1 ファイル 1.5 秒、初回は 10 秒を超える。

openpyxl は全シート読めるが、遅いうえに整形は呼び出し側まかせになる。

どちらも、レビューで問われること、つまり**その出力は差分として読めるのか**を見ていない。

## インストール

crates.io から入れる。

```console
$ cargo install oyomi
```

main を追いたい場合はリポジトリから入れる。

```console
$ cargo install --git https://github.com/HaiTo/oyomi
```

手を入れる場合はクローンしてから入れる。

```console
$ git clone https://github.com/HaiTo/oyomi
$ cd oyomi
$ cargo install --path .
```

どれも Rust のツールチェイン（[rustup](https://rustup.rs)）が要る。バイナリは `~/.cargo/bin` に入る。このディレクトリは `PATH` に通っている必要がある。git は textconv のドライバをシェルと同じやり方で探すため。

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

画像と埋め込みオブジェクトも一覧に出す。指紋にはパッケージが元から持っている CRC32 を使う。

```console
$ oyomi report.docx | grep '^media'
media	word/media/image1.png	48213	crc32:9a3f1c22

$ oyomi --row report.docx | grep '^media'
media	.png	48213	crc32:9a3f1c22
```

スクリーンショットを差し替えても本文は 1 文字も変わらないため、この行がないと、画像を抱えた文書は何も起きなかったかのような差分になる。`--row` がパート名を落とすのは、保存するだけで image1 と image2 が振り直されることがあり、そのとき画像自体は何も変わっていないため。

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

2 つのキーの使い分けが、このツールの設計のほとんどを占める。

既定は位置をキーにする。`Sheet!B62`、`p0042`、`t1.r003.c02` の形。探しものをするときはこちらが正しい。キーがそのままアプリケーションに打ち込むアドレスになる。

`--row` は内容だけをキーにする。差分を読むときはこちらが正しい。**位置は、1 つ挿入されただけで以降が全部変わる**ため。620 セルのシートに 3 行足した差分は、位置キーで 314 行、内容キーで 6 行。60 段落の文書に 3 段落足した差分は 93 行と 7 行になる。

```diff
+History	A=2	C=1.1	E=added three jobs
+Items	A=group-09	B=JOB900	C=batch	D=new job 900	E=yes
+Items	A=group-09	B=JOB901	C=batch	D=new job 901	E=yes
+Items	A=group-09	B=JOB902	C=batch	D=new job 902	E=yes
```

位置キーだけにして雑音を受け入れる案を採らない理由。3 行の変更に対する 314 行の差分は、レビュアーが読み飛ばせる雑音ではなく、誰も読まない差分になる。それはこのツールが解こうとしている状態そのもの。

行を識別する列からキーを作り、並べ替えにも耐えさせる案を採らない理由。どの列が識別子なのかを知っている必要があり、それは文書ごとに違うので、ツール側では決められない。

## 計測

実文書ではなく生成したサンプルでの数字。3 シート 637 セルの xlsx と、63 段落の docx。

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

テスト用の文書は全て `tests/common/mod.rs` で生成している。xlsx・docx・pptx のパッケージ、2 枚目のシートが宣言だけされていて実体のないブック、UTF-8 として不正なパート、どの読み手も開けないエントリを持つ手組みの zip まで作る。

フィクスチャをバイナリで置かない理由。バイナリを読めるようにするためのツールが、読めないバイナリを抱えるのは筋が通らない。誰も読めないフィクスチャは、誰も直せない。

テストが見ているのは差分の行数ではなく性質のほう。挿入のあと、内容キーでの旧出力が新出力の部分列になり、位置キーではならないこと。

行カバレッジは 99.6%。到達していない 1 箇所は、zip の中央ディレクトリが自分のエントリと食い違っている場合に備えた分岐。

## 未対応

数式を見ていない。calamine が返すのはキャッシュ済みの計算結果なので、式を書き換えても値が同じだと差分に出ない。

xlsx の図形内テキストとセルのコメントを拾っていない。

docx の変更履歴は `w:del` を `<del>` で囲むところまで。`w:ins` は素通ししている。

ノートはパッケージの関係定義ではなくパート名の番号でスライドに対応づけている。ノートの番号がスライドと独立に振られた資料では、対応先を取り違える。

## ライセンス

MIT
