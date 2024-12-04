# tt

ttはある時間まで、リクエストされた曲を流すための自動化ツールです。

# Setup

## API

### Google Form

まず適当にGoogleFormを作成します。Emailの収集をオンにして曲名とアーティスト名を聞けばいいと思いますが、質問の名前は必要に応じて変更可能です。Emailは個人の識別情報として回数の記録に使われます。
また曲名とアーティスト名はそのまま空白で繋がれて検索に使われるだけなので曲が一意に識別できそうなら他のものでも問題ありません。

そして、スプレッドシートと紐づけます。このとき列がそれぞれAからタイムスタンプ、メールアドレス、曲名、アーティスト名になっていることを確認してください。ただし曲名とアーティスト名は先述した理由により逆の順番でも構いません。
また5列目(デフォルトでは `E`)が空になっていることを確認してください。その列は自動で生成される値が入るようになります。

### Google Spreadsheetとの連携

#### スクリプトの登録

Formと連携したスプレッドシートのメニューの、拡張機能からApps Scriptを開いてください。そしてコードには[Releases](https://github.com/satler-git/tt/releases)からダウンロードした最新の安定版の `main.gs` の内容を入力してください。
このときに使った `main.gs` のバージョンとあとからダウンロードするttのバージョンが同じになるようにしてください。
そして右上のデプロイから新しいデプロイを押し以下のような設定でデプロイします。

![Screenshot 2024-12-04 09 04 42](https://github.com/user-attachments/assets/1cfeea29-99ef-4447-8e19-60ce4eb6988f)

このときにデプロイIDというものが出るので保存しておいてください(後で必要になります)。

#### トリガーの登録

![image](https://github.com/user-attachments/assets/0ca4b47d-df98-4aad-b5dc-41d80be6221a)

Apps Scriptの右端のメニューからトリガーを選び以下のスクリーンショットのように登録します。

![image](https://github.com/user-attachments/assets/db5a377d-9175-41de-8044-70bd10cef681)

これで5列目(デフォルトでは `E`)に自動的に生成されたユニークな値が入力されるようになります。

### Google API

Google Cloudにプロジェクトを登録してYoutube Search APIが使えるAPIキーを作成し記録しておきます。
スクリーンショットはないので頑張ってください。

## 本体

### 依存

動作には `mpv` `yt-dlp` `ffmpeg` が必要です。 `scoop` が既にインストールされている場合、以下のコマンドを実行することでインストールできます。

```powershell
scoop bucket add extras
scoop install main/ffmpeg main/yt-dlp extras/mpv
```

### 本体

[Releases](https://github.com/satler-git/tt/releases)から最新の安定版のWindows向けのバイナリをダウンロードし解凍します。

### 設定

一度ttを実行すると `%APPDATA%\Roaming\tt\tt.toml` に設定ファイルが生成されます。
`end_time` はデフォルトは `[13, 5, 0]` でこれは13時5分を過ぎたときに曲の再生が終わったらプログラムが終了するということです。
`gas_api_key` は先程のデプロイID、`youtube_api_key` はGoogle CloudのAPIキーです。

### 使い方

デフォルトの終了時間で終了して良いのならttを実行するだけで問題ありません。
またコマンドラインから 

- `--end_time` `-e` を指定してその後 `12:30` のように24時間表記で終了する時間を指定すればその時間になります
- `--duration` `-d` のあとに `1h5m5s` (`h` `m` `s` それぞれ省略可能) のように継続時間を指定すればその時間が立ったあとに終了します
- `--` を書いたあとにmpv向けの追加の引数を指定できます。デフォルトでmpvはフルスクリーンです
