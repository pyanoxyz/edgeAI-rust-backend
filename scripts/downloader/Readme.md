Usage:

## In url JSON mode

```bash
./downloader https://downloads.pyano.network/resources/manifests/test.json
```

json format:

```json
{
    "files": [
        {
            "name": "languages.so",
            "path": ".pyano-test/parsers/",
            "url": "https://downloads.pyano.network/resources/backend/parsers/languages.so"
        }
    ]
}
```

## In single file mode

```bash
./downloader https://downloads.pyano.network/resources/backend/parsers/languages.so .pyano-test/language.so
```

## Local JSON mode

```bash
./downloader ./path/to/local/manifest.json
```
