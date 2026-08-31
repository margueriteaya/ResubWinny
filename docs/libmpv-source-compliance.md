# libmpv 源代码与发布合规性

[简体中文](libmpv-source-compliance.md) · [繁體中文](libmpv-source-compliance.zh-TW.md) · [日本語](libmpv-source-compliance.ja.md) · [English](libmpv-source-compliance.en.md)

> **规范性说明：** 简体中文版本是唯一权威来源。其他语言版本仅为同步译文；如有歧义或冲突，以简体中文版本为准。

ResubWinny 在 Windows 上动态加载可替换的 LGPL 构建 libmpv。随附的开发 DLL 已固定并经哈希检查，但其上游构建使用了会变动的依赖分支，且未将完整源代码缓存作为发布制品公开。因此，当前 DLL 仅获准用于开发和私有测试，不得用于 ResubWinny 的首次公开二进制发布。

## 构建配置

基础 Tauri 配置不随附 libmpv 二进制文件。因此，它可以在不下载或再分发该库的情况下构建应用程序和安装包；此时预览需要用户提供兼容运行时。常规的一键构建使用显式 Windows libmpv 配置，以产生可用于开发/私有测试的包：

```powershell
./scripts/build.ps1
```

外部运行时包同样明确：

```powershell
./scripts/build.ps1 -Libmpv External
```

两种配置均不得削弱公开发布规则。含有 `libmpv-2.dll` 的包必须待与之匹配的完整对应源代码、源代码收据、声明和二进制哈希一并发布后，方可发布。只有随附 DLL 的哈希保持完全一致时，同一经验证的对应源代码制品才可供后续 ResubWinny 构建复用。

## 必需的公开发布构建

发布构建必须从 `third_party/versions.json` 中记录的提交运行，并且在下载或编译包之前应用已审查的仅 LGPL 补丁。同一个构建作业必须保留：

1. 精确的构建配方检出；
2. 已打补丁的 `mpv-winbuild-cmake` 检出；
3. 下载和更新后的完整 `src_packages` 目录；
4. 产生的 DLL 与导入库哈希；
5. 包提交/状态收据；
6. 重建所需的全部构建选项、补丁、许可证文本和脚本；以及
7. 已记录的 runner、工具链和原生包环境。

针对这三个源代码目录以及包含新构建 DLL/导入库的提取目录运行 `scripts/package-libmpv-source.ps1`，并传入同一作业产生的构建环境记录。该脚本拒绝缺少核心包的源代码缓存，验证固定配方与工具链祖先、LGPL 配置，记录每个源代码包，并创建以哈希寻址的对应源代码归档。

该归档和 `SOURCE-RECEIPT.json` 必须上传至每个含有该 libmpv DLL 的公开二进制文件旁。可变的仓库分支、GitHub Actions 运行 URL，或只包含 mpv 本身的源代码归档均不足够。

## 更新 libmpv

未来的 libmpv 更新只可作为一个不可分割的变更接受，其中包括：

- 新的二进制文件、导入库、头文件和哈希；
- 由同一构建生成的精确源代码归档与收据；
- 更新后的声明和 `third_party/versions.json`；
- 导出符号和可替换性检查；
- 预览、seek、叠加时钟、DPI 及 2K/4K/8K 性能回归；以及
- 明确的回滚制品。

任何运行时组件均不得下载或静默替换 libmpv。
