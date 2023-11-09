# n-nacos
  使用 `rust` 编写 `java` 的 `nacos`。

## 项目
```
├── packages                                         
│   ├── server                                       // server
├── commons                                          // 公共
│   ├── monitor                                      // 监控
│   └── utils                                        // utils
├── components                                       // 公共组件
├── .gitignore                                       // gitignore文件
├── .rustfmt.toml                                    // 格式化配置文件
├── Cargo.toml                                       // rust程序配置文件
└── README.md                                        // 项目使用说明文件
```

## 环境安装
   - protobuf
   ```shell
    brew install protobuf
    protoc --version
   ```