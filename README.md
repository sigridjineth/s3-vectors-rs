# S3 Vectors Rust SDK with RAG Demo
* https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-vectors.html
* https://github.com/awslabs/s3vectors-embed-cli

## Configuration

Set environment variables:
- `AWS_REGION` (default: us-east-1) - **Must be `us-east-1` or `us-west-2`**
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_SESSION_TOKEN` (optional)

## RAG Demo

This project includes a complete RAG implementation that:
- Uses Candle framework for BERT embeddings (all-MiniLM-L6-v2 model)
- Processes documents in parallel using Rayon
- Stores vectors in Amazon S3 Vectors
- Provides semantic search capabilities

### Running the RAG Demo

1. **Build the project**:
```bash
cargo build --release --example rag_demo
```

2. **Initialize the RAG pipeline** (creates S3 Vectors bucket and index):
```bash
cargo run --example rag_demo -- init
```

3. **Ingest documents**:
```bash
cargo run --example rag_demo -- ingest --directory test_documents
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running `target/debug/examples/rag_demo ingest --directory test_documents`
📄 Ingesting documents from: test_documents
2025-07-17T07:16:35.974147Z  INFO s3_vectors::rag: Starting document ingestion from: test_documents
2025-07-17T07:16:35.975614Z  INFO s3_vectors::document: Processed 2 documents from directory
2025-07-17T07:16:35.975647Z  INFO s3_vectors::rag: Found 2 documents to process
2025-07-17T07:16:35.982392Z  INFO s3_vectors::document: Split document doc-1 into 20 chunks
2025-07-17T07:16:35.982435Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(33)
2025-07-17T07:16:35.982456Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:16:35.982475Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:16:35.982631Z  INFO s3_vectors::document: Split document doc-0 into 23 chunks
2025-07-17T07:16:35.982646Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(31)
2025-07-17T07:16:35.982652Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:16:35.982663Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:17:23.901052Z  INFO s3_vectors::deploy: Putting 43 vectors to index documents-sigrid in bucket rag-demo-sigrid
2025-07-17T07:17:25.407469Z  INFO s3_vectors::deploy: Successfully put 43 vectors
2025-07-17T07:17:25.407660Z  INFO s3_vectors::rag: Total vectors uploaded: 43
2025-07-17T07:17:25.407905Z  INFO s3_vectors::rag: Document ingestion completed in 49.55480375s
✅ Document ingestion completed in 49.55486025s

```

4. **Query the system**:
```bash
cargo run --example rag_demo -- query --query "독자 AI 파운데이션 모델 프로젝트」 (총괄) 사업계획 신청서" --top-k 5
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s
     Running `target/debug/examples/rag_demo query --query '독자 AI 파운데이션 모델 프로젝트」 (총괄) 사업계획 신청서' --top-k 5`
🔍 Searching for: 독자 AI 파운데이션 모델 프로젝트」 (총괄) 사업계획 신청서

2025-07-17T07:45:54.549425Z  INFO s3_vectors::rag: Searching for: 독자 AI 파운데이션 모델 프로젝트」 (총괄) 사업계획 신청서
2025-07-17T07:45:54.549536Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(1)
2025-07-17T07:45:54.549554Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:45:54.549626Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:45:56.634915Z  INFO s3_vectors::deploy: Querying vectors in index documents-sigrid of bucket rag-demo-sigrid
2025-07-17T07:45:57.386262Z  INFO s3_vectors::rag: Found 5 relevant documents
Based on the retrieved context, here's a response to your query:

Query: 독자 AI 파운데이션 모델 프로젝트」 (총괄) 사업계획 신청서

Context Summary:
[Document 1]
----- | | | (단위 : 천원) | | 구 분 1차(\`25.下) 2차(\`26.上) 3차(\`26.下) 4차(\`27년) 합 계 금 액 % 금 액 % 금 액 % 금 액 % 정부출연금 현물 (GPU) 31,000,000 100% 37,200,000 100% 49,800,000 100% 99,터) 현금 (인재) 민 간 부 담 금 주관기관 현금 현물 참여1 기관명 현금 현물 참여2 기관명 현금 현물 참여3 기관명 현금 현물 계 현금 현물 계 합 계 | | ***※ 요약서는 5페이지 내...

[Document 2]
| | | | o o | | 참여기관1 | | | | o o | | 참여기관2 | | | | o o | **다. 기관별 과제수행 전문성** **1\) 주관기관 :** 기관명 | 설립년월일 | | 사업자번호 | | 홈페이지 | | | :---: | ----- | :---: | :--- | :---: | :---: | | 기업소개 (보유기술 포함 차별성, 혁신성, 추진 역량 등) | | | | | | | 주요연혁 | | | | | | | 주요사업 수행실적 | | | | | | **2\) 참여기관①\_(주)00000** \*참여기관별 작성 | 설립년월일 | | 사...

[Document 3]
진행 | \- 전국민 대상으로 웹 기반의 AI 서비스를 공개 및 OBT 테스트 진행 | 주관기관/ 참여기관1/ .... | 참여기관1/ 참여기관2/ .... | | | 멀티모달 API OBT 베타 테스트 공개 | \- 기업 및 공공 법인 대상 모기관2/ .... | | | 예비 창업/개발자 교육 | \- AI 관련 서비스 제작을 희망하는 예비 창업 및 개발 ...

[Document 4]
| ----- | ----- | ----- | ----- | ----- | ----- | | 주관기관 | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | | 참여기관 (1) | 책 임 자 | | | | | | | | | 실무책임자 | | | | | 정산담당자 | | | | | | | | 참여기관 (2) | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | | ... | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | **2\. 1...

[Document 5]
| :---- | :---- | **1\. 최종목표** ㅇ \- | AI 파운데이션 모델 최종 목표 | | | | | | | :---: | :---: | ----- | ----- | ----- | ----- | | **구분** | | **1차** **(\`25.下)** | **2차** **(\`26.上)** | **4차** **(\`27)** | | **개발** **목표** | | ㅇ Diffusion LLM 및 임베딩 모델 개념 증명 | ㅇ 멀티모달 Diffusion LLM(이미지, 텍스트) 개발 | ㅇ Diffusion LLM의 멀티모달(영상,음성) 및 추론 기능 추가 | ㅇ Diffusio...
```

5. **Interactive mode**:
```bash
 cargo run --example rag_demo -- interactive
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.38s
     Running `target/debug/examples/rag_demo interactive`
🤖 Interactive RAG Query Mode
Type 'exit' or 'quit' to stop

> 전국민 대상으로 OBT 테스트 해볼까요?
2025-07-17T07:46:28.128934Z  INFO s3_vectors::rag: Searching for: 전국민 대상으로 OBT 테스트 해볼까요?
2025-07-17T07:46:28.129138Z  INFO s3_vectors::embeddings: Loading BERT model on thread: ThreadId(1)
2025-07-17T07:46:28.129170Z  INFO s3_vectors::embeddings: Loading BERT model: sentence-transformers/all-MiniLM-L6-v2 (revision: main)
2025-07-17T07:46:28.129301Z  INFO s3_vectors::embeddings: Loading model from local files
2025-07-17T07:46:30.154958Z  INFO s3_vectors::deploy: Querying vectors in index documents-sigrid of bucket rag-demo-sigrid
2025-07-17T07:46:30.892279Z  INFO s3_vectors::rag: Found 5 relevant documents

Based on the retrieved context, here's a response to your query:

Query: 전국민 대상으로 OBT 테스트 해볼까요?

Context Summary:
[Document 1]
----- | | | (단위 : 천원) | | 구 분 1차(\`25.下) 2차(\`26.上) 3차(\`26.下) 4차(\`27년) 합 계 금 액 % 금 액 % 금 액 % 금 액 % 정부출연금 현물 (GPU) 31,000,000 100% 37,200,000 100% 49,800,000 100% 99,터) 현금 (인재) 민 간 부 담 금 주관기관 현금 현물 참여1 기관명 현금 현물 참여2 기관명 현금 현물 참여3 기관명 현금 현물 계 현금 현물 계 합 계 | | ***※ 요약서는 5페이지 내...

[Document 2]
| | | | o o | | 참여기관1 | | | | o o | | 참여기관2 | | | | o o | **다. 기관별 과제수행 전문성** **1\) 주관기관 :** 기관명 | 설립년월일 | | 사업자번호 | | 홈페이지 | | | :---: | ----- | :---: | :---: | :---: | :---: | | 기업소개 (보유기술 포함 차별성, 혁신성, 추진 역량 등) | | | | | | | 주요연혁 | | | | | | | 주요사업 수행실적 | | | | | | **2\) 참여기관①\_(주)00000** \*참여기관별 작성 | 설립년월일 | | 사...

[Document 3]
| ----- | ----- | ----- | ----- | ----- | ----- | | 주관기관 | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | | 참여기관 (1) | 책 임 자 | | | | | | | | | 실무책임자| | | | | | 참여기관 (2) | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | | ... | 책 임 자 | | | | | | | | | 실무책임자 | | | | | | | | | 정산담당자 | | | | | | | **2\. 1...

[Document 4]
| | 최근5년간 기업현황(단위: 천원,명) | | | | | | | | | | ----- | :---: | :---: | ----- | ----- | ----- | ----- | :---: | :---: | ----- | ----- | ----- | :---: | :---: | ----- | :---: | | | | | | | | | 구분 | 2021 | | 2022 | | 2023 | 2024 | | 2025 | | | | | | | | | 총자산 | | | | | | | | | | **설 립 년 월 일** | | | | | | | 총자본 | | | | | | | | | | **사업자등록번호** | | | | | | | 총부채 | |등록번호** | | | | | | | 매출액 | ...

[Document 5]
| | :---: | :---: | :---: | ----- | :---: | ----- | | 정부출연금 | | 현물(GPU\*) | 217,600,000 | 100% | 217,600,000 | | 민 간 부 담 금 | 주관기관명 | 현금 | | | | | | | 현물 | | | | | | 참여기관명 | | | | 참여기관명 | 현금 | | | | | | | 현물 | | | | | | 참여기관명 | 현금 | | | | | | | 현물 | | | | | | 계 | 현금 | | | | | | | 현물 | | | | | | | 계 | | | | | 합 계 | | | | | | 2\. 비목별 총괄 (단위 : 천원...


```