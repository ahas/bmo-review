[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# bmo-review

基于 git 检出目录进行代码审查的单一服务，带有自己的 API。它按设置中列出的检出目录，在某个修订版本上读取每一个
(目录树、文件、提交及每个提交的 diff、内容搜索和定义搜索)；管理针对修订版本的审查，以及挂在其中行、文件和文件夹上的
评论；并能从审查向作业服务提交条目。它从不向检出目录写入任何内容。对作业服务的访问只通过对方发布的客户端进行，
如 `docs/api.md` 所列。

该服务发布自己的客户端(`bmo_review::client`)、OpenAPI 文档(`docs/openapi.json`，由 `bmo-review openapi`
输出)以及 harness 插件：MCP 服务器 `bmo-review mcp` 和 `plugin/skills/` 下的 `bmo-review` 技能。

## 测试

个人数据规则(`PERSONAL-DATA.md`)由测试台的 `personal-data` 检查来保证。仓库包含以下测试：`tests/import.rs`
中的导入测试；`tests/screen.rs` 中的控制台入口测试，它按照客户端 vendor 中 bmo-ui 发行版的读取方式，通过 bmo-chat
的 `bmo-console-face`(使用机器上的 Bun)读取服务提供的清单，并确认服务的每条路由接受哪种凭据；以及
`tests/choices.rs` 中的界面选择检查，它确认界面中所有从列表里选择的值都使用 bmo-ui 唯一的 select 组件。控制台包
带有一个浏览器测试 `client/tests/browser/composer-frame.test.ts`：它在服务的替身之上，通过开发宿主
`client/dev-app/` 打开界面，并在两种配色方案下直接从页面上读取并确认：撰写评论的输入框在宽度 1280 的代码之间、
以及宽度 360 和 1280 的模态框中都只有一个边框；模态框在丢弃已写内容前会先询问，没有内容时立即关闭；在手机弹出键盘
那样矮的屏幕上，模态框的底部仍留在屏幕内(`docs/screen.md`，「Reviewing」)。测试台按客户端清单的声明在其浏览器
通道中运行它，包的类型检查通过 `client/tests/tsconfig.json` 读取其类型。其余的行为测试暂缓：等最小服务集在新仓库
上运转起来后再在这里编写和运行，旧行为需要满足的内容列在 `docs/inventory.md` 中。在此之前，构建检查为
`cargo check --all-targets` 和包的类型检查。

## 目录结构

| 文件夹 | 内容 |
|---|---|
| `src/` | 服务、客户端、OpenAPI 文档和 MCP 服务器，一个 crate 拆分为多个模块 |
| `migrations/` | 存储，按顺序执行这些语句来建表 |
| `plugin/` | harness 插件的技能 `bmo-review` |
| `client/` | 控制台包：代码审查界面、自带的韩语语言包、`bundle/` 下的构建、`dev-app/` 下的开发宿主以及浏览器测试，是一个独立项目 |
| `deploy/` | 服务的用户单元 |
| `docs/` | 描述服务行为的规范文档，以及 OpenAPI 文档 |
| `tests/` | 导入测试、控制台入口测试和界面选择检查 |

根目录下有 Cargo 清单、`LICENSE`、`VERSION`、`testbed.toml`、`rustfmt.toml`、`CLAUDE.md`、
`PERSONAL-DATA.md`、`korean-ledger.json`、`settings.toml`，以及部署时提供的本地设置示例
`settings.local.toml.example`。

## 控制台包

`client/` 将代码审查界面按照 bmo-ui 的控制台插件约定(bmo-ui `docs/console-bundles.md`)做成控制台包。它基于
bmo-chat 的 `shared-versions.toml` 锁定的 bmo-ui 发行版构建，该文件按发行命令写出的原样放在 `client/vendor/` 中。
`bun run bundle` 使用 bmo-ui 的包构建将其构建到 `client/dist/`，部署会在发行版上线前将其构建进发行版。服务通过
bmo-chat 的 `bmo-console-face` 提供该包(`docs/api.md`，「The console face」)，界面文案放在 `client/i18n/` 下
自己的语言包中。同一套源码也作为 Nuxt 项目进行类型检查，其类型由 `bun run api` 从 OpenAPI 文档生成。包的检查是在
`client/` 中运行 `bun install --frozen-lockfile && bun run typecheck`。

## 文档

| 文档 | 内容 |
|---|---|
| `docs/code-review.md` | 代码审查：项目、读取的修订版本、边界与排除项、搜索、审查、评论和提交 |
| `docs/screen.md` | 控制台界面的形态 |
| `docs/api.md` | 路由及其响应和代码、事件、对其他单元的调用、客户端、harness 插件、控制台入口和设置 |
| `docs/openapi.json` | 由服务的类型生成的 OpenAPI 3.1 文档，涵盖同样的路由、请求体和工具 |
| `docs/inventory.md` | 本仓库从旧仓库中接收的内容，以及每组的数量和来源 |
| `PERSONAL-DATA.md` | 本仓库中绝不允许出现的内容(个人的姓名、地址、路径、设备或网络信息，房间成员或公司的信息，真实对话中的任何一句)，以及保证这一点的检查 |

## 版本

`VERSION` 存放仓库的版本，也就是 crate 自身的版本，首次发行前为 `0.1.0-rev.<n>`。版本只在所有者宣布的发行时
变动(`docs/api.md`，「Versions」)。
