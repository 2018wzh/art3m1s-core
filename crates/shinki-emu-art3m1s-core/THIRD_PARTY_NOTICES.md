# Artemis 第三方说明

Art3m1s Core 来源于 Alphaly2K/art3m1s-core，采用 MPL-2.0。公开 fork 为 [`Project-Shinkirou/art3m1s-core`](https://github.com/Project-Shinkirou/art3m1s-core) 的 `shinki` 分支，基于上游 `ad90a62cb192b119b7cf8589aa7f87a94bd747c8`。本适配 crate 与核心同属该 fork，采用 MPL-2.0；完整许可证见仓库根 [LICENSE](../../LICENSE)。

其内部 PF8 库来自 sakarie9/pfs-rs，采用 MIT，保留 [原许可证](../pf8/LICENSE)。其他内部依赖继续保留各自许可证和归属，不因 Family 适配改变授权。

ShinkiEMU C ABI、SDK 与 host-side 构建辅助来自 [Project-Shinkirou/ShinkiEMU](https://github.com/Project-Shinkirou/ShinkiEMU)（`cb8d9dd22d7d0ab129085b324c94d6dc81fa19e3`），采用 MIT OR Apache-2.0。SDK 共享底层来自 [Project-Shinkirou/RoukakuEngine](https://github.com/Project-Shinkirou/RoukakuEngine)（`ba28948b5dd00188fe53a24881a64dcaf0c69e2a`）。CPAL 0.15.3 来自 [RustAudio/cpal](https://github.com/RustAudio/cpal)，采用 Apache-2.0，使用 [Project-Shinkirou/cpal@ecfda841a9140d60bbec1630d47ca91f1a77b8f3](https://github.com/Project-Shinkirou/cpal/blob/ecfda841a9140d60bbec1630d47ca91f1a77b8f3/LICENSE)。

Kira 0.12.2 来自 [tesselode/kira](https://github.com/tesselode/kira)，采用 MIT OR Apache-2.0；[原 MIT 许可证](https://github.com/Project-Shinkirou/kira/blob/7d8961b20bce2ab8fd43b4bb34150d7e2898785c/LICENSE-MIT)与[原 Apache-2.0 许可证](https://github.com/Project-Shinkirou/kira/blob/7d8961b20bce2ab8fd43b4bb34150d7e2898785c/LICENSE-APACHE)保留。native-owner 生命周期修复与精确来源记录在 MODIFICATIONS。

固定版本及本地差异见 [MODIFICATIONS.md](MODIFICATIONS.md)。商业游戏内容不属于本仓分发内容。
