# Artemis 第三方说明

Art3m1s Core 来源于 Alphaly2K/art3m1s-core，采用 MPL-2.0。公开 fork 为 [`2018wzh/art3m1s-core`](https://github.com/2018wzh/art3m1s-core) 的 `astra` 分支，基于上游 `ad90a62cb192b119b7cf8589aa7f87a94bd747c8`。本适配 crate 与核心同属该 fork，采用 MPL-2.0；完整许可证见仓库根 [LICENSE](../../LICENSE)。

其内部 PF8 库来自 sakarie9/pfs-rs，采用 MIT，保留 [原许可证](../pf8/LICENSE)。其他内部依赖继续保留各自许可证和归属，不因 Family 适配改变授权。

AstraEMU C ABI 与 SDK 来自 [2018wzh/AstraEngine](https://github.com/2018wzh/AstraEngine)（`6d9d691dcc8f3d768e40b5be1d6f37366cca62d8`），采用 MIT OR Apache-2.0。CPAL 0.15.3 来自 [RustAudio/cpal](https://github.com/RustAudio/cpal)，采用 Apache-2.0，使用 [2018wzh/cpal@ecfda841a9140d60bbec1630d47ca91f1a77b8f3](https://github.com/2018wzh/cpal/blob/ecfda841a9140d60bbec1630d47ca91f1a77b8f3/LICENSE)。

Kira 0.12.2 来自 [tesselode/kira](https://github.com/tesselode/kira)，采用 MIT OR Apache-2.0；[原 MIT 许可证](https://github.com/2018wzh/kira/blob/7d8961b20bce2ab8fd43b4bb34150d7e2898785c/LICENSE-MIT)与[原 Apache-2.0 许可证](https://github.com/2018wzh/kira/blob/7d8961b20bce2ab8fd43b4bb34150d7e2898785c/LICENSE-APACHE)保留。native-owner 生命周期修复与精确来源记录在 MODIFICATIONS。

固定版本及本地差异见 [MODIFICATIONS.md](MODIFICATIONS.md)。商业游戏内容不属于本仓分发内容。
