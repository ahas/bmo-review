[English](README.md) | **한국어** | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# bmo-review

git 체크아웃 위에서 코드 리뷰를 하는, 자체 API를 갖춘 하나의 서비스입니다. 설정에 지정된 체크아웃을 각각 특정
리비전에서 읽고(트리, 파일, 커밋과 커밋별 diff, 내용 검색과 정의 검색), 리비전에 대한 리뷰와 그 줄·파일·폴더에 단
코멘트를 다루며, 리뷰에서 작업 서비스로 항목을 등록합니다. 체크아웃에는 아무것도 쓰지 않습니다. 콘솔의 멤버에게는 에이전시가 그
계정의 것으로 기록한 프로젝트만 보입니다. 작업 서비스와 에이전시에는 `docs/api.md`에 나열된 대로 그쪽이 공개한
클라이언트를 통해서만 접근합니다.

이 서비스는 자체 클라이언트(`bmo_review::client`), OpenAPI 문서(`docs/openapi.json`, `bmo-review openapi`로
출력), 그리고 하네스 플러그인을 공개합니다. 하네스 플러그인은 MCP 서버 `bmo-review mcp`와 `plugin/skills/` 아래의
`bmo-review` 스킬로 이루어집니다.

## 테스트

개인정보 규칙(`PERSONAL-DATA.md`)은 테스트베드의 `personal-data` 검사가 지킵니다. 저장소에는 다음 테스트가
있습니다. `tests/import.rs`의 가져오기 테스트, `tests/screen.rs`의 콘솔 페이스 테스트, `tests/owners.rs`의 콘솔 읽는 사람 테스트(대역 에이전시를 두고, 멤버는
자기 계정이 소유한 프로젝트만, 관리자는 전부, 에이전시가 응답하지 않으면 멤버는 아무것도 읽지 못함을 확인), 그리고 `tests/choices.rs`의
화면 선택 검사입니다. 콘솔 페이스 테스트는 서비스가 제공하는 매니페스트를 클라이언트가 vendor로 둔 bmo-ui 릴리스의
읽기 방식 그대로 bmo-chat의 `bmo-console-face`(머신의 Bun)로 읽고, 서비스의 각 라우트가 어떤 자격 증명을 받는지
확인합니다. 화면 선택 검사는 화면에서 목록으로 고르게 하는 모든 값이 bmo-ui의 단일 select 안에 있는지 확인합니다.
콘솔 번들에는 브라우저 테스트 `client/tests/browser/composer-frame.test.ts`가 하나 있습니다. 이 테스트는 서비스의
대역(stand-in) 위에서 개발 호스트 `client/dev-app/`으로 화면을 열고, 두 색 구성 모두에서 다음을 페이지에서 직접
읽어 확인합니다. 코멘트를 쓰는 입력 상자가 1280 너비의 코드 사이에서, 그리고 360과 1280 너비의 모달 안에서 테두리를
하나만 갖는지, 모달이 작성한 글을 버리기 전에 묻고 글이 없으면 바로 닫히는지, 키보드가 올라온 휴대폰 정도로 짧은
화면에서도 모달 하단이 보이는지입니다(`docs/screen.md`, 「Reviewing」). 테스트베드는 클라이언트 매니페스트의 선언에
따라 이를 브라우저 레인에서 실행하고, 번들의 타입 검사는 `client/tests/tsconfig.json`으로 그 타입을 읽습니다.
그 밖의 동작 테스트는 미뤄 두었습니다. 최소 서비스들이 새 저장소에서 자리를 잡으면 여기서 작성하고 실행하며,
`docs/inventory.md`에 예전 동작이 지켜야 했던 내용이 정리되어 있습니다. 그때까지 빌드 검사는
`cargo check --all-targets`와 번들의 타입 검사입니다.

## 구성

| 폴더 | 내용 |
|---|---|
| `src/` | 서비스, 클라이언트, OpenAPI 문서, MCP 서버. 하나의 크레이트를 여러 모듈로 나눈 것 |
| `migrations/` | 저장소. 이 SQL 문들을 순서대로 실행해 테이블을 만듦 |
| `plugin/` | 하네스 플러그인의 스킬 `bmo-review` |
| `client/` | 콘솔 번들. 코드 리뷰 화면, 영어·한국어·일본어·중국어 간체로 된 자체 언어 팩, `bundle/` 아래의 빌드, `dev-app/` 아래의 개발 호스트, 브라우저 테스트를 담은 독립 프로젝트 |
| `deploy/` | 서비스의 사용자 유닛 |
| `docs/` | 서비스 동작에 대한 기준 문서와 OpenAPI 문서 |
| `tests/` | 가져오기 테스트, 콘솔 페이스 테스트, 화면 선택 검사 |

루트에는 Cargo 매니페스트, `LICENSE`, `testbed.toml`, `install.toml`, `rustfmt.toml`, `CLAUDE.md`,
`PERSONAL-DATA.md`, `korean-ledger.json`, `settings.toml`, 그리고 배포 시 주는 로컬 설정의 예시인
`settings.local.toml.example`이 있습니다.

## 콘솔 번들

`client/`는 코드 리뷰 화면을 bmo-ui의 콘솔 플러그인 규약(bmo-ui `docs/console-bundles.md`)에 따른 콘솔 번들로
담고 있습니다. bmo-chat의 `shared-versions.toml`이 고정한 bmo-ui 릴리스 위에 만들어지며, 그 파일은 릴리스 명령이
쓴 그대로 `client/vendor/`에 있습니다. `bun run bundle`은 bmo-ui의 번들 빌드로 이를 `client/dist/`에 빌드하고,
배포는 릴리스가 적용되기 전에 이를 릴리스 안에 빌드합니다. 서비스는 bmo-chat의 `bmo-console-face`를 통해 번들을
제공하며(`docs/api.md`, 「The console face」), 화면의 문구는 `client/i18n/` 아래의 자체 팩에 있습니다. 같은 소스는
Nuxt 프로젝트로서 타입 검사를 받으며, 그 타입은 `bun run api`로 OpenAPI 문서에서 생성합니다. 번들의 검사는
`client/` 안에서 실행하는 `bun install --frozen-lockfile && bun run typecheck`입니다.

## 문서

| 문서 | 내용 |
|---|---|
| `docs/code-review.md` | 코드 리뷰: 프로젝트, 읽는 리비전, 경계와 제외 대상, 검색, 리뷰, 코멘트와 등록 |
| `docs/screen.md` | 콘솔 화면의 구성 |
| `docs/api.md` | 라우트와 그 응답·코드, 이벤트, 다른 유닛 호출, 클라이언트, 하네스 플러그인, 콘솔 페이스와 설정 |
| `docs/openapi.json` | 같은 라우트, 본문, 도구를 서비스의 타입에서 생성한 OpenAPI 3.1 문서 |
| `docs/inventory.md` | 이 저장소가 예전 저장소에서 가져온 것과 묶음별 개수 및 출처 |
| `PERSONAL-DATA.md` | 이 저장소에 절대 들어가면 안 되는 것(사람의 이름, 주소, 경로, 기기나 네트워크 정보, 방 구성원이나 회사의 정보, 실제 대화의 한 줄)과 이를 지키는 검사 |

## 버전

버전은 세트의 버전이며, 빌드할 때 git 태그에서 읽습니다. 크레이트 매니페스트의 버전은 움직이지 않는 자리표시자입니다.
버전은 소유자가 선언한 릴리스에서만 바뀝니다(`docs/api.md`, 「Versions」).
