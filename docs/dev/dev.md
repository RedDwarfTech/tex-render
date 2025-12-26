## 初始化模版编译依赖文件夹

新建模版编译依赖文件夹：

```bash
mkdir -R ~/texmf/tex/latex
```

初始化模版编译依赖：

```bash
cp -r resource/tex/* ~/texmf/tex/latex
```

## 从服务端同步字体

```bash
scp -r tencent01:/opt/tex/texmf/tex/latex/Font .
```

部分简历编译时，需要自定义的字体。字体存放在源码的`config/font`路径下。远程服务器的`/opt/tex/texmf/tex/latex/Font`路径下。


## 从服务端同步cls

```bash
scp -r tencent01:/home/ubuntu/texmf/tex/ .
```

简历编译时需要自定义的cls，cls本地存储在源码的config/tex路径下，远程服务器的`~/texmf/tex/latex/`路径下。

## 将字体拷贝到系统路径下

简历编译时，有部分自定义字体，如果提示不存在时，请确认已经将字体将自定义字体拷贝到服务端系统路径`/usr/share/fonts/opentype/reddwarfcv`下。reddwarfcv表示红矮星简历(Red Dwarf )字体目录。

```bash
sudo cp -R /opt/tex/texmf/tex/latex/Font/*.otf .
sudo find /opt/tex/texmf/tex/latex/Font/ -type f -name "*.otf" -exec cp {} /usr/share/fonts/opentype/reddwarfcv \;
```

将字体文件拷贝到系统路径`/usr/share/fonts/truetype/reddwarfcv`下：

```bash
sudo cp -R /opt/tex/texmf/tex/latex/Font/*.ttf .
sudo find /opt/tex/texmf/tex/latex/Font/ -type f -name "*.ttf" -exec cp {} /usr/share/fonts/truetype/reddwarfcv \;
sudo find /home/ubuntu/IBM_Plex_Serif/ -type f -name "*.ttf" -exec cp {} /usr/share/fonts/truetype/reddwarfcv \;
```