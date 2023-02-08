# これは何

イクシエの連絡帳を一度にまとめてエクスポートするツールです。

# 使い方

## Windows

Windowsの場合、Windowsセキュリティがixsie-exporterをトロイの木馬として認識してしまうため、これを解除する必要があります。下のステップ2からステップ8までが解除操作に当たります。

1. [Windows用実行ファイル（zipファイル）](https://github.com/maoe/ixsie-exporter/releases/download/v0.1.0/ixsie-exporter-x86_64-pc-windows-msvc.zip)をダウンロード
2. Windowsセキュリティの警告のポップアップをクリック。もし表示されなければステップ9へ。
  ![screenshot 01](images/screenshot-01.png)
3. Windowsセキュリティに表示されている脅威のうち 「Trojan Script/Wacatac.H!ml」 をクリック
  ![screenshot 02](images/screenshot-02.png)
4. 「詳細を表示」をクリック
  ![screenshot 03](images/screenshot-03.png)
5. 「このアプリがデバイスに変更を加えることを許可しますか？」の質問に「はい」と答える
  ![screenshot 04](images/screenshot-04.png)
6. 「影響を受けた項目」 に「ixsie-exporter」の文字列が含まれていることを確認して「OK」する
  ![screenshot 05](images/screenshot-05.png)
7. 「デバイスで許可」を選択して「操作の開始」を押す
  ![screenshot 06](images/screenshot-06.png)
8. 再度「このアプリがデバイスに変更を加えることを許可しますか？」 に「はい」と答える
  ![screenshot 07](images/screenshot-07.png)
9. ブラウザのダウンロード一覧から「ixsie-exporter-x86_64-pc-windows-msvc.zip」をクリックする
  ![screenshot 08](images/screenshot-08.png)
10. エクスプローラー上部ピンク色の「展開」を選択し、「すべて展開」を押す。
  ![screenshot 09](images/screenshot-09.png)
11. 「完了時に展開されたファイルを表示する」にチェックを入れて「展開」する。新しく開いたエクスプローラーに「ixsie-expoter」が表示される。
  ![screenshot 10](images/screenshot-10.png)
12. Windowsのスタートメニューに「cmd」と入力してコマンドプロンプトを起動する
  ![screenshot 11](images/screenshot-11.png)
13. コマンドプロンプトに「cd Desktop」と入力してエンターを押す
  ![screenshot 12](images/screenshot-12.png)
14. 先ほど展開した「ixsie-exporter」をコマンドプロンプトにドラッグアンドドロップして、エンターを押す。
  ![screenshot 13](images/screenshot-13.png)
15. 「Login E-mail address:」と表示される。イクシエのログインメールアドレスを入力してエンターを押す。
  ![screenshot 14](images/screenshot-14.png)
16. 「Login password:」と表示される。イクシエのログインパスワードを入力してエンターを押す。ログインに成功するとダウンロードが始まる。コマンドプロンプトにはダウンロードが終わった順に「年-月」が表示される。
  ![screenshot 15](images/screenshot-15.png)
17. すべて終わったら「Windowsセキュリティ」を開き「ウイルスの脅威と防止」をクリックして、「許可された脅威」をクリック。先ほど許可した「Trojan:Script/Wacatac.H!ml」を選択して「許可しない」を押す。

以上で終了です。ダウンロードしたPDFはデスクトップにあります。

## macOS
