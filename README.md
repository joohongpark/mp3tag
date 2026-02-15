# mp3tag

MP3 파일의 ID3 태그를 편집하는 CLI/GUI 유틸리티
Spotify API를 통해 파일의 메타데이터를 업데이트 할 수 있음.

## 기능

- MP3 파일의 ID3v2.4 태그 읽기/쓰기
- 디렉토리 재귀 스캔 및 태그 현황 조회
- 파일명 패턴 파싱으로 아티스트/제목 자동 추출
- Spotify 검색으로 태그 및 앨범 아트 자동 적용
- CLI (clap) / GUI (egui) 두 가지 인터페이스 지원

## 빌드

```bash
# CLI + GUI (기본)
cargo build --release

# CLI만
cargo build --release --no-default-features
```

## 사용법

### Spotify 설정

Spotify Developer Dashboard(https://developer.spotify.com/dashboard)에서 앱을 생성하고 Client ID/Secret을 발급받은 후:

```bash
mp3tag config
```

실행하면 자격증명을 입력받아 `config.toml`에 저장한다.

### CLI 명령어

```bash
# 디렉토리 스캔 (태그 현황 테이블 출력)
mp3tag scan <디렉토리>

# 수동 태그 편집
mp3tag edit <파일> --title "제목" --artist "아티스트" --album "앨범"

# Spotify에서 태그 검색 및 적용 (태그 없는 파일 대상)
mp3tag fetch <파일 또는 디렉토리>
```

### GUI 모드

```bash
mp3tag --gui [디렉토리]
```

## 프로젝트 구조

```
mp3tag/
├── Cargo.toml
├── config.toml              # Spotify 자격증명 설정 파일
├── src/
│   ├── main.rs              # 엔트리포인트
│   ├── cli.rs               # clap 명령어 정의 및 CLI 핸들러
│   ├── config.rs            # 설정 파일 로드/저장
│   ├── models.rs            # 공유 데이터 모델 (TrackInfo, Mp3File)
│   ├── core/
│   │   ├── mod.rs
│   │   ├── scanner.rs       # 디렉토리 스캔, MP3 파일 탐색
│   │   ├── tagger.rs        # ID3 태그 읽기/쓰기
│   │   └── parser.rs        # 파일명 -> 아티스트/제목 파싱
│   ├── sources/
│   │   ├── mod.rs           # MusicSource 트레이트 정의
│   │   └── spotify.rs       # Spotify Web API 클라이언트
│   └── gui/
│       ├── mod.rs           # GUI 실행 진입점
│       └── app.rs           # egui 앱 (파일 목록, 태그 편집, 검색)
```

## 주요 의존성

| 용도 | 크레이트 |
|------|---------|
| ID3 태그 | `id3` |
| CLI | `clap` |
| GUI | `eframe`, `egui` |
| HTTP | `reqwest` |
| 직렬화 | `serde`, `serde_json`, `toml` |
| 폴더 선택 | `rfd` |
| 이미지 | `image` |
| 에러 처리 | `anyhow` |

## 확장

`sources/mod.rs`의 `MusicSource` 트레이트를 구현하면 Bugs, Melon 등 추가 소스를 연동할 수 있다.
