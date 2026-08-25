//! Exact-plan-bound trusted fleet Principal successor cutover.
//! This is intentionally an offline one-shot operator, not a reassignment API.
#![allow(unexpected_cfgs)]
use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{collections::HashSet, env, fs, process};
use svc_workflow::domain::workflow_instance::recovery::{BeforeSnapshotV1, WorkflowProjection};
use uuid::Uuid;

const PLAN_SHA: &str = "0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606";
const PLAN_SIZE: usize = 540472;
const PLAN_PATH: &str =
    "/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json";
const EVENT_TYPE: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED";
const COMMAND_TYPE: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_V1";
const AUDIT_ACTION: &str = "PRINCIPAL_SUCCESSOR_MIGRATION_V1_COMMITTED";
const EXCLUDED_AGENT: &str = "efficiency-agent";
const EXCLUDED_PRINCIPAL: &str = "d09f8849-073c-484a-978c-f375113c28b2";
const PAIRS: [(&str, &str, &str, &str); 86] = [
    (
        "ceo-agent",
        "agt_ceo-agent",
        "26b2be56-353f-406c-9865-bff91149b4fb",
        "25a6789f-daa5-4600-a764-b0209b9c8e19",
    ),
    (
        "stock-agent",
        "agt_stock-agent",
        "b09f1417-d26c-4f77-a3ac-8dc4fb4a18f9",
        "8e299ff0-2e28-4998-a062-3a880c65096c",
    ),
    (
        "research-agent",
        "agt_research-agent",
        "1e0bebef-4150-4a24-ad42-aaccded084a2",
        "a4b14810-321b-4060-ae75-042dcb87892b",
    ),
    (
        "knowledge-curator-agent",
        "agt_knowledge-curator-agent",
        "87047adb-2931-400b-b5f0-c384cba37b8d",
        "8402d851-5bae-475c-bdb2-6fa1522803cf",
    ),
    (
        "daily-thought-agent",
        "agt_daily-thought-agent",
        "2a24b855-4640-48ea-9042-56235e451bf1",
        "4074e8c2-67f1-409f-830c-1690cc7c64f2",
    ),
    (
        "efficiency-manager",
        "agt_efficiency-agent",
        "95eab282-22c7-46a2-8580-abfef4942cdc",
        "b21ddb23-42f6-47c4-a27f-bc44950e554c",
    ),
    (
        "lobster-agent",
        "agt_lobster-agent",
        "ccb6cb22-f45b-46bf-959a-87ab2c472bdb",
        "d2c98674-03b1-4b87-bba2-e41eddb0568a",
    ),
    (
        "itops-agent",
        "agt_itops-agent",
        "eee90beb-dfcc-49dd-9dbb-fa6b3db73a72",
        "04dd2c02-502e-4a1e-bc21-7719ab42d718",
    ),
    (
        "healthcheck-agent",
        "agt_healthcheck-agent",
        "25012773-1b80-4018-bc5d-053fc0fcc30f",
        "9758e1d7-fc63-4e7f-9277-ef9020bf923c",
    ),
    (
        "hr-agent",
        "agt_hr-agent",
        "bc970ced-710f-4479-9ff0-e295a1c59424",
        "dc702687-6515-4a2a-91ae-e572a9bbd766",
    ),
    (
        "security-agent",
        "agt_security-agent",
        "af8708f0-eb99-409b-87c9-7c7ee596b967",
        "e15a185f-e82d-49d5-9316-04ebb5b0a251",
    ),
    (
        "skill-engineer-agent",
        "agt_skill-engineer-agent",
        "45f817cc-50ff-4c57-9897-0fd5dc3d2d6a",
        "8eac00cf-fd18-456b-85a5-717ae9e463b4",
    ),
    (
        "discipline-coach-agent",
        "agt_discipline-coach-agent",
        "f1e3fcac-f94b-4845-9c0f-272d66b3c816",
        "ddff9f0a-6ef0-4bf0-aff9-239ee8e38c8c",
    ),
    (
        "blog-agent",
        "agt_blog-agent",
        "81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6",
        "fd58881a-fdba-4ef2-9a80-b733671f24f1",
    ),
    (
        "education-agent",
        "agt_education-agent",
        "137992b6-e09f-40ca-84a3-1f797078ef58",
        "debec784-3bd9-4dcf-aca4-549a7416d680",
    ),
    (
        "psychology-agent",
        "agt_psychology-agent",
        "0477c74b-cb51-47ce-a2ee-5e359b1d7f04",
        "c4f53ae3-c2b9-4eee-a542-f13469b38705",
    ),
    (
        "game-dev-agent",
        "agt_game-dev-agent",
        "913dbd48-2311-4a10-9620-54c8b3e1c61a",
        "6d184076-6037-4d29-96bc-b0196bab5056",
    ),
    (
        "finance-agent",
        "agt_finance-agent",
        "3f99bf95-95c5-437f-bb76-990a88ab97af",
        "3e6c1225-a306-423b-91da-4ed3b9acdd1b",
    ),
    (
        "devtools-agent",
        "agt_devtools-agent",
        "1680fd97-e4c4-42df-9ce6-ba46c9dc31e2",
        "7dadf79c-0144-4212-9695-535d854c88dc",
    ),
    (
        "voice-tech-agent",
        "agt_voice-tech-agent",
        "32f44973-4aca-4b3f-9668-65bde6e5c234",
        "90aac9b8-c1a4-496c-aed7-fe42c08023eb",
    ),
    (
        "image-gen-agent",
        "agt_image-gen-agent",
        "87b760da-14b3-46d1-b529-9350ad95d2c0",
        "bebd919a-9da5-46a0-a0de-57625066ddab",
    ),
    (
        "email-manager-agent",
        "agt_email-manager-agent",
        "446ac233-db75-4086-ba67-d87445bc3f41",
        "af948833-5e48-4965-b1df-fdd565d1d63c",
    ),
    (
        "account-manager-agent",
        "agt_account-manager-agent",
        "bba89c1f-734b-4fc8-95b0-f1a1bdf9b30d",
        "a36ab7b7-490a-4753-b98f-c412959d4b56",
    ),
    (
        "shopping-list-agent",
        "agt_shopping-list-agent",
        "4466ab5c-3a3f-454b-8fe3-a6ff11ad5c1f",
        "fbc89748-6c1c-46cd-8ccf-bfa8b5853a75",
    ),
    (
        "feishu-expert-agent",
        "agt_feishu-expert-agent",
        "029e5deb-81ca-4f9e-a7a6-f92a7e4bc0a6",
        "1e0e3be2-82ee-4bcf-818f-96048664b84f",
    ),
    (
        "podcast-producer-agent",
        "agt_podcast-producer-agent",
        "b75d9e79-4c17-4546-8ff9-1dbafe57e144",
        "16f9a572-4281-4602-9ec6-22a7687378e7",
    ),
    (
        "soul-questioner-agent",
        "agt_soul-questioner-agent",
        "502ce1a2-a141-4782-971b-d60f4efe244d",
        "e5947aa1-7a42-4a56-95aa-9af5601852fe",
    ),
    (
        "lobster-guide-agent",
        "agt_lobster-guide-agent",
        "af31460d-2b2a-4730-b1df-ee5d19e1a876",
        "5ee12519-49bd-4fbc-a626-c9f754f15d09",
    ),
    (
        "article-publisher-agent",
        "agt_article-publisher-agent",
        "d2bec623-56ba-46cb-a217-933cfe63a243",
        "8b44befd-934e-48cd-afac-31ed981fb732",
    ),
    (
        "travel-planner-agent",
        "agt_travel-planner-agent",
        "41dfb084-cc74-4c07-9f51-f526b89dabf7",
        "254db36c-6a88-4356-9dee-f5ee1695841c",
    ),
    (
        "agent-dev-engineer",
        "agt_agent-dev-engineer",
        "39938395-a46c-46e8-b558-849de147f877",
        "1e716670-f3b0-44fd-a9da-9ed46fc65789",
    ),
    (
        "paper-reviewer-agent",
        "agt_paper-reviewer-agent",
        "1338fd22-3bed-4f1d-91a9-ae85e90ad7f0",
        "9004627e-6c97-4eb7-8031-51d881b7722e",
    ),
    (
        "3d-print-agent",
        "agt_3d-print-agent",
        "30ec9983-32b1-4ffb-80df-6edd1d24e3ef",
        "ed55e35c-f0a1-43fe-a2db-f6b7c45b7005",
    ),
    (
        "writing-style-analyst-agent",
        "agt_writing-style-analyst-agent",
        "61819256-07e1-4bd0-adea-e93e51243fa1",
        "9e3adced-575f-4fb2-b351-f7698b59127d",
    ),
    (
        "family-doctor-2-agent",
        "agt_family-doctor-2-agent",
        "a45858d2-f72a-47c8-b75a-cb5bf1d4aee3",
        "3affffb7-feb9-4fe7-b865-9fee4d760fb1",
    ),
    (
        "feishu-expert-2-agent",
        "agt_feishu-expert-2-agent",
        "fe04ccdf-5902-4536-a4ac-22cc633cbd00",
        "ab1cd4a3-5b01-45ea-a165-20a97b4af87f",
    ),
    (
        "reimbursement-expert",
        "agt_reimbursement-expert",
        "64bbfc4c-c842-4d2d-bf97-a5c3cd7376da",
        "86f138b7-43cf-42ec-8694-9e32be0f0862",
    ),
    (
        "mobile-app-engineer",
        "agt_mobile-app-engineer",
        "50042be0-57ef-4f37-a8d0-d936eddf24d1",
        "858da6a2-ccbd-49e1-bc94-1afceb92d423",
    ),
    (
        "miniapp-game-engineer",
        "agt_miniapp-game-engineer",
        "fe836dbd-896f-40f0-a74f-9c273d5ac233",
        "d2691e77-4ce4-493d-96e8-870e56f4977e",
    ),
    (
        "trend-tracker",
        "agt_trend-tracker",
        "c05d2cba-837f-41f0-a7db-f587550eace2",
        "4bb26368-52ec-4def-82c0-3a64400c7818",
    ),
    (
        "biz-explorer",
        "agt_biz-explorer",
        "fe2bfbbb-229f-483d-aaa1-42ae14b79a49",
        "9ddbb1c7-ea91-43af-832d-59033375cb7f",
    ),
    (
        "video-producer",
        "agt_video-producer",
        "96db2152-2dc7-4eb3-89ad-6c532dd9e625",
        "94c55acc-8b93-46f9-81af-9f28382f8fbc",
    ),
    (
        "creative-writer",
        "agt_creative-writer",
        "3c4b92a3-fe95-407c-8363-5b44b78d46f6",
        "622cc051-9388-4e41-9f0d-7b8cec035b0e",
    ),
    (
        "test-engineer",
        "agt_test-engineer",
        "93c7cade-4f0a-47aa-bb6e-7d31a3a147f8",
        "033ea97d-296b-4cd1-a92a-eb638f42d49f",
    ),
    (
        "learning-expert",
        "agt_learning-expert",
        "6ccdae57-0a2c-4a2e-b3e4-e3f4ec1edf2a",
        "40daa67b-af5a-462d-b4ca-d40213147afb",
    ),
    (
        "content-ops-agent",
        "agt_content-ops-agent",
        "aed2c679-71c0-4e23-8d73-c448c5ea6dd2",
        "271876c1-5bc9-4902-855d-68bf5c99eccb",
    ),
    (
        "finance-housekeeper-agent",
        "agt_finance-housekeeper-agent",
        "018f6e50-9744-4dd1-b005-51a66a305db8",
        "5a647224-fa7e-490f-91fe-a134daf37479",
    ),
    (
        "quant-trading-agent",
        "agt_quant-trading-agent",
        "14d8845d-d59c-428b-b0e4-4d3e88497f54",
        "fddf93dd-fbe2-4664-8e09-26601bdef58f",
    ),
    (
        "novel-writer",
        "agt_novel-writer",
        "f3b8bb2d-8963-4416-80ee-3d167e196391",
        "d15420ee-90fa-4f52-8114-b66b4a6c2f98",
    ),
    (
        "frontend-react-engineer",
        "agt_frontend-react-engineer",
        "54922e8e-9ce1-41b9-bea4-9caca051c0eb",
        "f62285fc-e36d-4d16-9de5-8cd869f410c8",
    ),
    (
        "open-source-agent",
        "agt_open-source-agent",
        "00e0b5ef-487d-4eea-8999-7be1309ace70",
        "9edc2b13-c543-4776-acdc-b7a7474ce2cb",
    ),
    (
        "smart-home-agent",
        "agt_smart-home-agent",
        "3c80e5b1-6951-49a2-b448-5ad5bea0a2cd",
        "ea95eef8-b7c8-4fa7-b73d-e51839d44a44",
    ),
    (
        "product-manager",
        "agt_product-manager",
        "e4215c47-de83-455b-9e12-53edc983cf76",
        "33a71ab4-0eee-4bb7-86fe-a513cac7cbe0",
    ),
    (
        "product-designer",
        "agt_product-designer",
        "968bff25-17f5-4e72-8bb9-5e2238b4bbbc",
        "3d4eb075-235c-4ad7-8abd-a8e8d36d879d",
    ),
    (
        "qa-reviewer",
        "agt_qa-reviewer",
        "d1ffc337-6ecf-494a-89ce-53b16a95dee3",
        "18f52e76-3922-47d1-92d7-cec00ce8ac8d",
    ),
    (
        "investment-debater",
        "agt_investment-debater",
        "d602cde4-e2e1-4f07-b92b-8131d4999013",
        "69c92a47-6c42-4e72-9617-b887fb871b57",
    ),
    (
        "backend-engineer-2",
        "agt_backend-engineer-2",
        "b2b67eed-38b6-4aed-899b-b2b93fc56f80",
        "3fd24642-b6c9-4fcb-8d63-59acd47baefe",
    ),
    (
        "qa-reviewer-2",
        "agt_qa-reviewer-2",
        "f8c9c7c6-dee4-455b-8be7-2286e5967b1e",
        "1fa1e12d-24fb-47da-ac7b-fb4ab3eae080",
    ),
    (
        "social-butterfly-agent",
        "agt_social-butterfly-agent",
        "5f2a2010-2634-4391-a012-0cecd567eb81",
        "05459bc9-abca-4632-a297-c1c7cb97ddde",
    ),
    (
        "arch-reviewer",
        "agt_arch-reviewer",
        "4684680a-60c4-487b-b362-de7a3367fed2",
        "9df952bc-237e-4dc9-845f-cd52776bf25f",
    ),
    (
        "explorer",
        "agt_explorer",
        "c1c359a0-df35-41db-ae91-a1e26b44fa53",
        "5b9a9cb8-a027-42b7-a3b3-17d7e7afe5f7",
    ),
    (
        "ppt-designer",
        "agt_ppt-designer",
        "5a23fa35-4783-4bdd-a7b0-736e92acf974",
        "b2d51a5e-a2e5-4a2c-951d-d24b892ac2ce",
    ),
    (
        "training-expert-agent",
        "agt_training-expert-agent",
        "17b9f2b8-e8e8-4ef0-ad67-f36da95a9b2b",
        "41f0eadc-0774-4063-8c24-ca6dbc122398",
    ),
    (
        "needs-radar-agent",
        "agt_needs-radar-agent",
        "0c6a8e58-3275-4d40-a099-811832523cc4",
        "92e1d6d0-e722-41c9-b68b-90d97faa830f",
    ),
    (
        "delivery-review-agent",
        "agt_delivery-review-agent",
        "6bbb8dcb-62e2-4550-a59c-a5f13ba7d056",
        "823bf0e2-0c25-49c7-87ad-c77405c6c133",
    ),
    (
        "course-community-agent",
        "agt_course-community-agent",
        "1f5b6d46-4abd-4964-9575-1ccad219a1b2",
        "aaaf53e6-140b-4af4-9e34-82d0e6c92f2d",
    ),
    (
        "biz-product-designer",
        "agt_biz-product-designer",
        "ff90d51e-1a67-47ab-aaa9-67f6852a579b",
        "01599df6-d9aa-4e98-874c-77f6729f5941",
    ),
    (
        "private-chef-agent",
        "agt_private-chef-agent",
        "9b1c0122-df68-4aa1-a599-68c4416c4f56",
        "88e5cb26-349a-42e0-afed-bf741dbadc42",
    ),
    (
        "course-community-agent-2",
        "agt_course-community-agent-2",
        "132ab857-35ab-408b-b909-bc0b1deab55b",
        "9f7cf4c5-7b2c-4239-9993-d9b2a2e0df56",
    ),
    (
        "book-deconstructor-agent",
        "agt_book-deconstructor-agent",
        "1aa584d1-28c1-4260-8c21-015f47f34b87",
        "4b25917d-93df-496f-8715-afc9d734abc3",
    ),
    (
        "build-in-public-agent",
        "agt_build-in-public-agent",
        "bb9d8f48-7962-4321-8fb1-554bb428c159",
        "d5b3aeb2-e754-49a9-9914-b963521c0985",
    ),
    (
        "job-watch-agent",
        "agt_job-watch-agent",
        "5464f426-e91e-45e9-af21-85a1233d7bae",
        "30527120-2894-4f87-9516-302059149ddd",
    ),
    (
        "search-expert-agent",
        "agt_search-expert-agent",
        "935d0d15-3bd5-489b-a5ba-d577257c2638",
        "2fae18ce-c81e-411d-b9cc-76346d37ab5c",
    ),
    (
        "transcript-editor-agent",
        "agt_transcript-editor-agent",
        "66efbc9f-c618-47ba-b9da-0668cf5c7301",
        "79e4a43b-002b-4a03-90ca-82ee4724c287",
    ),
    (
        "home-repair-agent",
        "agt_home-repair-agent",
        "34a1263c-1a2b-4e90-a94e-a7f4d9c3ce70",
        "a8e69d20-c784-4f4a-aac9-6a24e5302717",
    ),
    (
        "sales-copy-agent",
        "agt_sales-copy-agent",
        "7702461e-b31d-493e-b6df-739d98bbc92c",
        "28588c07-82b6-4212-aa65-e633a93723bc",
    ),
    (
        "hao-yang-mao-agent",
        "agt_hao-yang-mao-agent",
        "03bd2421-1bd7-41a1-9ef8-709d03519aca",
        "8a81aa51-bbaa-4c93-9510-eb4f0da7a06c",
    ),
    (
        "family-steward-agent",
        "agt_family-steward-agent",
        "cd9243a4-8550-4869-b77a-eb30036cfb0f",
        "e4c0ca12-eaf5-406a-91b2-47ef07c0de9b",
    ),
    (
        "video-model-expert",
        "agt_video-model-expert",
        "e272c87d-8093-40e3-b285-6bacd9c74c9f",
        "08c18ba3-c594-43b1-9493-6b3e22b6bbbe",
    ),
    (
        "game-designer-agent",
        "agt_game-designer-agent",
        "4af8d4e7-8eae-4904-a142-b3f3d8f0b959",
        "4f16b012-4bc9-45ae-ab9d-6418f9a01fd4",
    ),
    (
        "game-producer-agent",
        "agt_game-producer-agent",
        "3b35907e-f3a5-4ce9-b09d-fcedd59cee94",
        "208f91e9-90da-4109-bfa7-710d8713212e",
    ),
    (
        "reader-simulator-agent",
        "agt_reader-simulator-agent",
        "ecdb0a53-d79c-4c7e-9b66-5e9436500e81",
        "93130a60-3c59-4ab9-88b5-70f152451e5b",
    ),
    (
        "thesis-advisor-agent",
        "agt_thesis-advisor-agent",
        "097f197d-dc71-4107-8db2-f2bdc8eb7ec5",
        "72ff7ee1-9260-485d-8400-5768f2b59acd",
    ),
    (
        "biz-reviewer",
        "agt_biz-reviewer",
        "fedef046-3289-4b0c-83c1-f7681373dce0",
        "3ca56da2-f00a-4e5e-8779-653be8057b4f",
    ),
    (
        "translator-agent",
        "agt_translator-agent",
        "1bd07594-230c-4e39-8b98-0e6895db52e1",
        "75707797-542b-44fe-be0e-38c0f11512b6",
    ),
    (
        "translation-qa-agent",
        "agt_translation-qa-agent",
        "64ccdc42-98f3-4b2a-b1ff-4e81b6ee8b03",
        "6bbdfbcc-7a6f-4925-ae70-07eeccf74d65",
    ),
];

#[derive(Debug)]
struct Error(String);
type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self(format!("database conflict: {e}"))
    }
}
fn conflict(s: impl Into<String>) -> Error {
    Error(format!("CONFLICT: {}", s.into()))
}
fn outcome_error(outcome: &str, s: impl Into<String>) -> Error {
    Error(format!("{outcome}: {}", s.into()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Plan,
    Apply(Scope),
    Verify,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scope {
    Build,
    Efficiency,
    Remaining,
}
#[derive(Debug)]
struct Cli {
    mode: Mode,
}

#[derive(Debug, Deserialize)]
struct Plan {
    schema: String,
    artifact_revision: u32,
    mode: String,
    production_change: String,
    summary: Summary,
    fleet_rows: Vec<Pair>,
    domain_tuples: Vec<DomainTuple>,
    current_responsibility_tuples: Vec<Responsibility>,
    creator_owned_draft_tuples: Vec<Value>,
    explicit_exclusions: Vec<Value>,
    excluded_identities: Value,
}
#[derive(Debug, Deserialize)]
struct Summary {
    total_new_agents: usize,
    exact_successor_pair_count: usize,
    ambiguous_count: usize,
    conflict_count: usize,
    new_workflow_projection_missing_count: usize,
    new_workflow_projection_present_count: usize,
    total_domain_owner_transfers: usize,
    total_domain_member_transfers: usize,
    total_active_responsibility_transfers: usize,
    total_draft_ownership_candidates: usize,
}
#[derive(Clone, Debug, Deserialize)]
struct Pair {
    old_agent_id: String,
    new_agent_id: String,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    old_principal_external_ref: String,
    new_principal_external_ref: String,
    old_principal_status: String,
    new_principal_status: String,
    classification: String,
    evidence: PairEvidence,
}
#[derive(Clone, Debug, Deserialize)]
struct PairEvidence {
    new_workflow_principal_present: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainTuple {
    domain_id: Uuid,
    domain_key: String,
    role: String,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    enabled: bool,
    migration_candidate: bool,
    before: DomainBefore,
    after: DomainAfter,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainBefore {
    old_enabled: bool,
    new_enabled: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct DomainAfter {
    old_enabled: bool,
    new_enabled: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct Responsibility {
    workflow_instance_id: Uuid,
    domain_id: Uuid,
    current_visit_id: Uuid,
    node_id: Uuid,
    old_principal_id: Uuid,
    new_principal_id: Uuid,
    expected_state_version: i32,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let message = e.to_string();
        let outcome = message
            .split_once(':')
            .map(|(prefix, _)| prefix)
            .filter(|value| matches!(*value, "CONFLICT" | "ROLLED_BACK" | "OUTCOME_UNKNOWN"))
            .unwrap_or("CONFLICT");
        println!("{}", json!({"outcome":outcome,"writes":0,"error":message}));
        process::exit(1);
    }
}
async fn run() -> Result<()> {
    let cli = parse_args()?;
    let (plan, raw) = load_plan()?;
    let database_url =
        env::var("DATABASE_URL").map_err(|_| conflict("DATABASE_URL is required"))?;
    let auth_url =
        env::var("AUTH_DATABASE_URL").map_err(|_| conflict("AUTH_DATABASE_URL is required"))?;
    let workflow = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;
    let auth = PgPoolOptions::new()
        .max_connections(2)
        .connect(&auth_url)
        .await?;
    verify_database_identity(&workflow, &auth).await?;
    let actor = env::var("MIGRATION_ACTOR_PRINCIPAL_ID")
        .ok()
        .map(|s| {
            s.parse::<Uuid>()
                .map_err(|_| conflict("MIGRATION_ACTOR_PRINCIPAL_ID must be UUID"))
        })
        .transpose()?;
    match cli.mode {
        Mode::Plan => observe(&workflow, &auth, &plan, None, false).await,
        Mode::Verify => observe(&workflow, &auth, &plan, None, true).await,
        Mode::Apply(scope) => {
            apply_scope(
                &workflow,
                &auth,
                &plan,
                scope,
                actor.ok_or_else(|| {
                    conflict("MIGRATION_ACTOR_PRINCIPAL_ID is required for apply")
                })?,
                &raw,
            )
            .await
        }
    }
}
#[cfg(not(trusted_fleet_cutover_conformance))]
async fn verify_database_identity(workflow: &PgPool, auth: &PgPool) -> Result<()> {
    let workflow_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(workflow)
        .await?;
    let auth_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(auth)
        .await?;
    if workflow_name != "svc_workflow_dogfood_clean" || auth_name != "agent_dev_center" {
        return Err(conflict(format!(
            "database identity mismatch: workflow={workflow_name}, auth={auth_name}"
        )));
    }
    Ok(())
}
#[cfg(trusted_fleet_cutover_conformance)]
async fn verify_database_identity(_workflow: &PgPool, _auth: &PgPool) -> Result<()> {
    Ok(())
}

fn parse_args() -> Result<Cli> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [flag] if flag == "--plan" => Mode::Plan,
        [flag] if flag == "--verify" => Mode::Verify,
        [apply, scope_flag, scope] if apply == "--apply" && scope_flag == "--scope" => {
            Mode::Apply(match scope.as_str() {
                "build-in-public-canary" => Scope::Build,
                "efficiency-canary" => Scope::Efficiency,
                "remaining-fleet" => Scope::Remaining,
                _ => return Err(conflict("invalid closed --scope")),
            })
        }
        _ => return Err(conflict("use only --plan, --verify, or --apply --scope <closed-scope>; arbitrary IDs, paths, and subsets are forbidden")),
    };
    Ok(Cli { mode })
}
fn load_plan() -> Result<(Plan, Vec<u8>)> {
    let raw = fs::read(PLAN_PATH)?;
    if raw.len() != PLAN_SIZE || hex::encode(Sha256::digest(&raw)) != PLAN_SHA {
        return Err(conflict("plan bytes do not match frozen size/SHA-256"));
    }
    let root: Value =
        serde_json::from_slice(&raw).map_err(|e| conflict(format!("invalid plan JSON: {e}")))?;
    let required = [
        "artifact_revision",
        "authority_recommendation",
        "broker_4xx_diagnosis",
        "classification_rules",
        "creator_owned_draft_tuples",
        "current_responsibility_tuples",
        "domain_tuples",
        "excluded_identities",
        "explicit_exclusions",
        "fleet_rows",
        "history_visibility",
        "mode",
        "production_change",
        "schema",
        "snapshot_observed_at_utc",
        "sources",
        "summary",
        "supersession",
    ];
    let obj = root
        .as_object()
        .ok_or_else(|| conflict("plan root must be object"))?;
    if obj.len() != required.len() || required.iter().any(|k| !obj.contains_key(*k)) {
        return Err(conflict("plan root has non-exact shape"));
    }
    let p: Plan = serde_json::from_value(root).map_err(|e| conflict(format!("plan shape: {e}")))?;
    validate_plan(&p)?;
    Ok((p, raw))
}
fn validate_plan(p: &Plan) -> Result<()> {
    if p.schema != "workflow_trusted_fleet_successor_plan_v2"
        || p.artifact_revision != 2
        || p.mode != "READ_ONLY_CANONICAL_PLAN"
        || p.production_change != "NONE"
    {
        return Err(conflict("plan identity fields drifted"));
    }
    let s = &p.summary;
    if (
        s.total_new_agents,
        s.exact_successor_pair_count,
        s.ambiguous_count,
        s.conflict_count,
        s.new_workflow_projection_missing_count,
        s.new_workflow_projection_present_count,
        s.total_domain_owner_transfers,
        s.total_domain_member_transfers,
        s.total_active_responsibility_transfers,
        s.total_draft_ownership_candidates,
    ) != (86, 86, 0, 0, 85, 1, 8, 752, 80, 99)
        || p.fleet_rows.len() != 86
        || p.domain_tuples.len() != 760
        || p.current_responsibility_tuples.len() != 80
        || p.creator_owned_draft_tuples.len() != 99
    {
        return Err(conflict("plan counts drifted"));
    }
    let mut ids = HashSet::new();
    for (i, (f, c)) in p.fleet_rows.iter().zip(PAIRS).enumerate() {
        if f.classification != "EXACT_SUCCESSOR_PAIR"
            || f.old_agent_id != c.0
            || f.new_agent_id != c.1
            || f.old_principal_id.to_string() != c.2
            || f.new_principal_id.to_string() != c.3
            || !ids.insert((f.old_principal_id, f.new_principal_id))
        {
            return Err(conflict(format!(
                "pair {} differs from compiled allowlist",
                i + 1
            )));
        }
    }
    if p.domain_tuples
        .iter()
        .filter(|x| x.role == "DOMAIN_OWNER")
        .count()
        != 8
        || p.domain_tuples
            .iter()
            .filter(|x| x.role == "DOMAIN_MEMBER")
            .count()
            != 752
        || p.domain_tuples.iter().any(|x| {
            !x.enabled
                || !x.migration_candidate
                || !x.before.old_enabled
                || x.before.new_enabled
                || x.after.old_enabled
                || !x.after.new_enabled
        })
    {
        return Err(conflict("domain tuple shape/state invalid"));
    }
    let pair_ids: HashSet<(Uuid, Uuid)> = p
        .fleet_rows
        .iter()
        .map(|pair| (pair.old_principal_id, pair.new_principal_id))
        .collect();
    if p.domain_tuples
        .iter()
        .any(|tuple| !pair_ids.contains(&(tuple.old_principal_id, tuple.new_principal_id)))
    {
        return Err(conflict("domain tuple is outside exact pair allowlist"));
    }
    let mut instances = HashSet::new();
    if p.current_responsibility_tuples.iter().any(|tuple| {
        !pair_ids.contains(&(tuple.old_principal_id, tuple.new_principal_id))
            || tuple.expected_state_version < 1
            || !instances.insert(tuple.workflow_instance_id)
    }) {
        return Err(conflict(
            "responsibility tuple is invalid, duplicate, or outside allowlist",
        ));
    }
    if p.creator_owned_draft_tuples
        .iter()
        .any(|tuple| tuple.get("migration_candidate").and_then(Value::as_bool) != Some(false))
    {
        return Err(conflict("draft creator tuple attempted migration"));
    }
    if p.explicit_exclusions.len() != 8 {
        return Err(conflict("explicit exclusions count drifted"));
    }
    let excluded_records = p
        .excluded_identities
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| conflict("excluded identity records missing"))?;
    if excluded_records.len() != 1
        || excluded_records[0].get("agent_id").and_then(Value::as_str) != Some(EXCLUDED_AGENT)
        || excluded_records[0]
            .get("principal_id")
            .and_then(Value::as_str)
            != Some(EXCLUDED_PRINCIPAL)
        || excluded_records[0]
            .get("future_operator_writes")
            .and_then(Value::as_i64)
            != Some(0)
    {
        return Err(conflict("excluded identity is not exact zero-write record"));
    }
    Ok(())
}
fn selected(scope: Scope, p: &Pair) -> bool {
    match scope {
        Scope::Build => p.new_agent_id == "agt_build-in-public-agent",
        Scope::Efficiency => p.new_agent_id == "agt_efficiency-agent",
        Scope::Remaining => {
            p.new_agent_id != "agt_build-in-public-agent"
                && p.new_agent_id != "agt_efficiency-agent"
        }
    }
}
async fn auth_exact(auth: &PgPool, p: &Pair) -> Result<String> {
    let rows=sqlx::query("SELECT id, principal_type::text AS principal_type, agent_id, external_ref, status::text AS status, display_name FROM machine_principals WHERE id = ANY($1) ORDER BY id").bind(&[p.old_principal_id,p.new_principal_id][..]).fetch_all(auth).await?;
    if rows.len() != 2 {
        return Err(conflict(format!(
            "Auth pair missing for {}",
            p.new_agent_id
        )));
    }
    let mut new_name = None;
    for r in rows {
        let id: Uuid = r.get("id");
        let principal_type: String = r.get("principal_type");
        let agent: Option<String> = r.try_get("agent_id").ok();
        let ext: Option<String> = r.try_get("external_ref").ok();
        let status: String = r.get("status");
        let name: Option<String> = r.try_get("display_name").ok();
        let (ea, ee, es) = if id == p.old_principal_id {
            (
                &p.old_agent_id,
                &p.old_principal_external_ref,
                &p.old_principal_status,
            )
        } else if id == p.new_principal_id {
            new_name = name.clone();
            (
                &p.new_agent_id,
                &p.new_principal_external_ref,
                &p.new_principal_status,
            )
        } else {
            return Err(conflict("Auth returned unplanned UUID"));
        };
        if principal_type != "agent"
            || agent.as_deref() != Some(ea)
            || ext.as_deref().unwrap_or("") != ee
            || status != *es
        {
            return Err(conflict(format!("Auth identity drift for {id}")));
        }
    }
    let n = new_name
        .ok_or_else(|| conflict(format!("Auth display_name missing for {}", p.new_agent_id)))?;
    if n.is_empty() || n.chars().count() > 256 {
        return Err(conflict("Auth display_name invalid"));
    }
    Ok(n)
}
async fn projection_exact(w: &PgPool, pair: &Pair, display: &str) -> Result<bool> {
    let row = sqlx::query("SELECT principal_type::text AS t,display_name,enabled FROM principals WHERE principal_id=$1")
        .bind(pair.new_principal_id)
        .fetch_optional(w)
        .await?;
    Ok(row.as_ref().is_some_and(|r| {
        r.get::<String, _>("t") == "AGENT"
            && r.get::<String, _>("display_name") == display
            && r.get::<bool, _>("enabled")
    }))
}

async fn domain_tuple_state(w: &PgPool, tuple: &DomainTuple) -> Result<&'static str> {
    let row = sqlx::query(
        "SELECT d.domain_key, d.enabled AS domain_enabled,
         (SELECT enabled FROM domain_role_bindings WHERE domain_id=d.domain_id AND principal_id=$2 AND role_key=$4) AS old_enabled,
         (SELECT enabled FROM domain_role_bindings WHERE domain_id=d.domain_id AND principal_id=$3 AND role_key=$4) AS new_enabled
         FROM domains d WHERE d.domain_id=$1",
    )
    .bind(tuple.domain_id)
    .bind(tuple.old_principal_id)
    .bind(tuple.new_principal_id)
    .bind(&tuple.role)
    .fetch_optional(w)
    .await?
    .ok_or_else(|| conflict(format!("domain {} is missing", tuple.domain_id)))?;
    if row.get::<String, _>("domain_key") != tuple.domain_key
        || !row.get::<bool, _>("domain_enabled")
    {
        return Ok("CONFLICT");
    }
    let old_enabled: Option<bool> = row.try_get("old_enabled").ok();
    let new_enabled: Option<bool> = row.try_get("new_enabled").ok();
    Ok(match (old_enabled, new_enabled) {
        (Some(true), None | Some(false)) => "PRE",
        (Some(false), Some(true)) => "READY",
        _ => "CONFLICT",
    })
}

async fn responsibility_tuple_state(w: &PgPool, tuple: &Responsibility) -> Result<&'static str> {
    let row = sqlx::query(
        "SELECT wi.domain_id, wi.current_node_visit_id, wi.workflow_state_version, wi.cancelled, wi.archived_at,
         v.node_id, v.assignee_principal_id, n.node_type::text AS node_type
         FROM workflow_instances wi
         JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id
         JOIN workflow_node_definitions n ON n.node_id=v.node_id AND n.definition_version_id=wi.definition_version_id
         WHERE wi.workflow_instance_id=$1",
    )
    .bind(tuple.workflow_instance_id)
    .fetch_optional(w)
    .await?
    .ok_or_else(|| conflict(format!("instance {} is missing", tuple.workflow_instance_id)))?;
    if row.get::<Uuid, _>("domain_id") != tuple.domain_id {
        return Ok("CONFLICT");
    }
    let current_visit: Uuid = row.get("current_node_visit_id");
    let version: i32 = row.get("workflow_state_version");
    let node: Uuid = row.get("node_id");
    let assignee: Option<Uuid> = row.try_get("assignee_principal_id").ok();
    let terminal = row.get::<bool, _>("cancelled")
        || row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("archived_at")
            .ok()
            .flatten()
            .is_some()
        || row.get::<String, _>("node_type") == "TERMINAL";
    if terminal {
        let source_ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$1 AND workflow_instance_id=$2 AND node_id=$3 AND assignee_principal_id=$4",
        )
        .bind(tuple.current_visit_id)
        .bind(tuple.workflow_instance_id)
        .bind(tuple.node_id)
        .bind(tuple.old_principal_id)
        .fetch_one(w)
        .await?;
        return Ok(if source_ok == 1 {
            "TERMINAL"
        } else {
            "CONFLICT"
        });
    }
    if current_visit == tuple.current_visit_id
        && version == tuple.expected_state_version
        && node == tuple.node_id
        && assignee == Some(tuple.old_principal_id)
    {
        return Ok("PRE");
    }
    if version == tuple.expected_state_version + 1
        && node == tuple.node_id
        && assignee == Some(tuple.new_principal_id)
    {
        let event_ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1 AND event_type=$2 AND source_node_visit_id=$3 AND target_node_visit_id=$4 AND old_workflow_state_version=$5 AND new_workflow_state_version=$6",
        )
        .bind(tuple.workflow_instance_id)
        .bind(EVENT_TYPE)
        .bind(tuple.current_visit_id)
        .bind(current_visit)
        .bind(tuple.expected_state_version)
        .bind(tuple.expected_state_version + 1)
        .fetch_one(w)
        .await?;
        return Ok(if event_ok == 1 { "READY" } else { "CONFLICT" });
    }
    Ok("CONFLICT")
}

async fn observe(
    w: &PgPool,
    a: &PgPool,
    p: &Plan,
    _scope: Option<Scope>,
    terminal: bool,
) -> Result<()> {
    let mut pairs = Vec::new();
    let mut projection_ready = 0usize;
    let mut owner_ready = 0usize;
    let mut member_ready = 0usize;
    let mut responsibility_ready = 0usize;
    let mut terminal_skipped = 0usize;
    let mut conflicts = 0usize;
    for pair in &p.fleet_rows {
        let mut pair_conflicts = Vec::new();
        let display = match auth_exact(a, pair).await {
            Ok(value) => value,
            Err(error) => {
                pair_conflicts.push(error.to_string());
                String::new()
            }
        };
        let projection = !display.is_empty() && projection_exact(w, pair, &display).await?;
        let projection_present: i64 =
            sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1")
                .bind(pair.new_principal_id)
                .fetch_one(w)
                .await?;
        if projection {
            projection_ready += 1;
        } else if projection_present != 0 {
            pair_conflicts
                .push("projection exists but does not exactly match Auth formal fields".into());
        } else if pair.evidence.new_workflow_principal_present {
            pair_conflicts.push("plan-required existing projection is now missing".into());
        } else if terminal {
            pair_conflicts.push("projection is not exact terminal state".into());
        }
        for tuple in p.domain_tuples.iter().filter(|tuple| {
            tuple.old_principal_id == pair.old_principal_id
                && tuple.new_principal_id == pair.new_principal_id
        }) {
            let state = domain_tuple_state(w, tuple).await?;
            if state == "READY" {
                if tuple.role == "DOMAIN_OWNER" {
                    owner_ready += 1;
                } else {
                    member_ready += 1;
                }
            } else if state == "CONFLICT" || terminal {
                if state != "READY" {
                    pair_conflicts.push(format!(
                        "domain tuple {} {} is {state}",
                        tuple.domain_id, tuple.role
                    ));
                }
            }
        }
        for tuple in p.current_responsibility_tuples.iter().filter(|tuple| {
            tuple.old_principal_id == pair.old_principal_id
                && tuple.new_principal_id == pair.new_principal_id
        }) {
            let state = responsibility_tuple_state(w, tuple).await?;
            match state {
                "READY" => responsibility_ready += 1,
                "TERMINAL" => {
                    terminal_skipped += 1;
                    responsibility_ready += 1;
                }
                "CONFLICT" => pair_conflicts.push(format!(
                    "responsibility {} drifted",
                    tuple.workflow_instance_id
                )),
                "PRE" if terminal => pair_conflicts.push(format!(
                    "responsibility {} remains in pre-state",
                    tuple.workflow_instance_id
                )),
                _ => {}
            }
        }
        if !pair_conflicts.is_empty() {
            conflicts += 1;
        }
        pairs.push(json!({"newAgentId":pair.new_agent_id,"outcome":if pair_conflicts.is_empty(){if terminal{"NOOP"}else{"PLANNED"}}else{"CONFLICT"},"projectionExact":projection,"drift":pair_conflicts}));
    }
    let excluded: Uuid = EXCLUDED_PRINCIPAL.parse().expect("compiled excluded UUID");
    let excluded_domain_writes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled",
    )
    .bind(excluded)
    .fetch_one(w)
    .await?;
    let excluded_visits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_node_visits WHERE assignee_principal_id=$1",
    )
    .bind(excluded)
    .fetch_one(w)
    .await?;
    if excluded_domain_writes != 0 || excluded_visits != 0 {
        conflicts += 1;
    }
    println!(
        "{}",
        json!({
            "planSha256": PLAN_SHA,
            "outcome": if conflicts == 0 { if terminal {"NOOP"} else {"PLANNED"} } else {"CONFLICT"},
            "writes": 0,
            "newAudits": 0,
            "workflowProjectionReadyCount": projection_ready,
            "domainOwnerReadyCount": owner_ready,
            "domainMemberReadyCount": member_ready,
            "activeResponsibilityReadyCount": responsibility_ready,
            "terminalResponsibilitySkippedCount": terminal_skipped,
            "conflictCount": conflicts,
            "errorCount": 0,
            "outcomeUnknownCount": 0,
            "historyRewriteCount": 0,
            "excludedIdentityDomainCount": excluded_domain_writes,
            "excludedIdentityVisitCount": excluded_visits,
            "pairs": pairs
        })
    );
    if conflicts == 0 {
        Ok(())
    } else {
        Err(conflict(format!("{conflicts} pair(s) have tuple drift")))
    }
}
async fn gate(w: &PgPool, a: &PgPool, plan: &Plan, agent: &str, actor: Uuid) -> Result<()> {
    let pair = plan
        .fleet_rows
        .iter()
        .find(|pair| pair.new_agent_id == agent)
        .ok_or_else(|| conflict("compiled canary missing"))?;
    let display = auth_exact(a, pair).await?;
    let migration_id = pair_migration_id(pair)?;
    if !pair_terminal_exact(w, plan, pair, &display, actor, &migration_id).await? {
        return Err(conflict(format!(
            "canary {agent} is not exact terminal PASS"
        )));
    }
    Ok(())
}
async fn apply_scope(
    w: &PgPool,
    a: &PgPool,
    p: &Plan,
    scope: Scope,
    actor: Uuid,
    _raw: &[u8],
) -> Result<()> {
    let actor_ok: i64 =
        sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1 AND enabled")
            .bind(actor)
            .fetch_one(w)
            .await?;
    if actor_ok != 1
        || actor.to_string() == EXCLUDED_PRINCIPAL
        || p.fleet_rows
            .iter()
            .any(|pair| actor == pair.old_principal_id || actor == pair.new_principal_id)
    {
        return Err(conflict(
            "operator Principal is absent, disabled, excluded, or part of the fleet",
        ));
    }
    if scope != Scope::Build {
        gate(w, a, p, "agt_build-in-public-agent", actor).await?;
    }
    if scope == Scope::Remaining {
        gate(w, a, p, "agt_efficiency-agent", actor).await?;
    }
    let mut outcomes = Vec::new();
    for (idx, pair) in p
        .fleet_rows
        .iter()
        .enumerate()
        .filter(|(_, x)| selected(scope, x))
    {
        let display = auth_exact(a, pair).await?;
        let outcome = apply_pair(w, p, idx + 1, pair, &display, actor).await?;
        outcomes.push(json!({"pairIndex":idx+1,"newAgentId":pair.new_agent_id,"outcome":outcome}));
    }
    let committed = outcomes
        .iter()
        .filter(|item| item.get("outcome").and_then(Value::as_str) == Some("COMMITTED"))
        .count();
    let noops = outcomes
        .iter()
        .filter(|item| item.get("outcome").and_then(Value::as_str) == Some("NOOP"))
        .count();
    println!(
        "{}",
        json!({"planSha256":PLAN_SHA,"scope":format!("{:?}",scope),"outcome":if committed==0{"NOOP"}else{"COMMITTED"},"writes":if committed==0{Some(0)}else{None},"newAudits":committed,"noopCount":noops,"pairs":outcomes})
    );
    Ok(())
}
fn pair_migration_id(pair: &Pair) -> Result<String> {
    Ok(format!(
        "trusted-fleet-v1:{}",
        digest(&json!({
            "planFileSha256": PLAN_SHA,
            "oldPrincipal": pair.old_principal_id,
            "newPrincipal": pair.new_principal_id,
        }))?
    ))
}
fn deterministic(label: &str) -> Uuid {
    let mut b: [u8; 16] = Sha256::digest(format!("{PLAN_SHA}:{label}").as_bytes())[..16]
        .try_into()
        .unwrap();
    b[6] = (b[6] & 0x0f) | 0x50;
    b[8] = (b[8] & 0x3f) | 0x80;
    Uuid::from_bytes(b)
}
fn digest(v: &Value) -> Result<String> {
    let s = jcs_canonicalize::canonicalize(
        &serde_json::to_string(v).map_err(|e| conflict(e.to_string()))?,
    )
    .map_err(|e| conflict(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(s.as_bytes())))
}
async fn pair_pre_exact(
    pool: &PgPool,
    plan: &Plan,
    pair: &Pair,
    display: &str,
    actor: Uuid,
    migration_id: &str,
) -> Result<bool> {
    let projected = projection_exact(pool, pair, display).await?;
    let projection_absent: i64 =
        sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1")
            .bind(pair.new_principal_id)
            .fetch_one(pool)
            .await?;
    if !projected && projection_absent != 0 {
        return Ok(false);
    }
    for tuple in plan.domain_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if domain_tuple_state(pool, tuple).await? != "PRE" {
            return Ok(false);
        }
    }
    for tuple in plan.current_responsibility_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if !matches!(
            responsibility_tuple_state(pool, tuple).await?,
            "PRE" | "TERMINAL"
        ) {
            return Ok(false);
        }
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3")
        .bind(actor).bind(format!("{migration_id}:summary")).bind(COMMAND_TYPE).fetch_one(pool).await?;
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_security_audits WHERE principal_id=$1 AND action=$2 AND resource_type='TRUSTED_FLEET_PRINCIPAL_CUTOVER' AND resource_id=$3")
        .bind(actor).bind(AUDIT_ACTION).bind(migration_id).fetch_one(pool).await?;
    Ok(receipts == 0 && audits == 0)
}

async fn pair_terminal_exact(
    pool: &PgPool,
    plan: &Plan,
    pair: &Pair,
    display: &str,
    actor: Uuid,
    migration_id: &str,
) -> Result<bool> {
    if !projection_exact(pool, pair, display).await? {
        return Ok(false);
    }
    for tuple in plan.domain_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if domain_tuple_state(pool, tuple).await? != "READY" {
            return Ok(false);
        }
    }
    for tuple in plan.current_responsibility_tuples.iter().filter(|tuple| {
        tuple.old_principal_id == pair.old_principal_id
            && tuple.new_principal_id == pair.new_principal_id
    }) {
        if !matches!(
            responsibility_tuple_state(pool, tuple).await?,
            "READY" | "TERMINAL"
        ) {
            return Ok(false);
        }
    }
    let summary: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3 AND receipt_status='COMPLETED'",
    )
    .bind(actor)
    .bind(format!("{migration_id}:summary"))
    .bind(COMMAND_TYPE)
    .fetch_one(pool)
    .await?;
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_security_audits WHERE principal_id=$1 AND action=$2 AND resource_type='TRUSTED_FLEET_PRINCIPAL_CUTOVER' AND resource_id=$3",
    )
    .bind(actor)
    .bind(AUDIT_ACTION)
    .bind(migration_id)
    .fetch_one(pool)
    .await?;
    Ok(summary == 1 && audits == 1)
}

async fn apply_pair(
    pool: &PgPool,
    plan: &Plan,
    index: usize,
    pair: &Pair,
    display: &str,
    actor: Uuid,
) -> Result<&'static str> {
    let migration_id = pair_migration_id(pair)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(&migration_id)
        .execute(&mut *tx)
        .await?;
    let summary_key = format!("{migration_id}:summary");
    let done:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2 AND command_type=$3 AND receipt_status='COMPLETED'").bind(actor).bind(&summary_key).bind(COMMAND_TYPE).fetch_one(&mut *tx).await?;
    if done > 1 {
        return Err(conflict(format!(
            "duplicate summary receipts for pair {index}"
        )));
    }
    if done == 1 {
        tx.rollback().await?;
        return if pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await? {
            Ok("NOOP")
        } else {
            Err(conflict(format!(
                "receipt exists but terminal state drifted for pair {index}"
            )))
        };
    }
    let old:i64=sqlx::query_scalar("SELECT count(*) FROM principals WHERE principal_id=$1 AND principal_type='AGENT' AND enabled").bind(pair.old_principal_id).fetch_one(&mut *tx).await?;
    if old != 1 {
        return Err(conflict(format!(
            "OLD Workflow projection drift pair {index}"
        )));
    }
    match sqlx::query("SELECT principal_type::text AS t,display_name,enabled FROM principals WHERE principal_id=$1 FOR UPDATE").bind(pair.new_principal_id).fetch_optional(&mut *tx).await?{None=>{sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,email,enabled,metadata) VALUES($1,'AGENT',$2,NULL,TRUE,NULL)").bind(pair.new_principal_id).bind(display).execute(&mut *tx).await?;},Some(r)=>if r.get::<String,_>("t")!="AGENT"||r.get::<String,_>("display_name")!=display||!r.get::<bool,_>("enabled"){return Err(conflict(format!("NEW Workflow projection drift pair {index}")));}}
    let domains: Vec<_> = plan
        .domain_tuples
        .iter()
        .filter(|x| {
            x.old_principal_id == pair.old_principal_id
                && x.new_principal_id == pair.new_principal_id
        })
        .collect();
    for d in &domains {
        let key = if d.role == "DOMAIN_OWNER" {
            "DOMAIN_OWNER"
        } else if d.role == "DOMAIN_MEMBER" {
            "DOMAIN_MEMBER"
        } else {
            return Err(conflict("unplanned role"));
        };
        let dk: String = sqlx::query_scalar(
            "SELECT domain_key FROM domains WHERE domain_id=$1 AND enabled FOR UPDATE",
        )
        .bind(d.domain_id)
        .fetch_one(&mut *tx)
        .await?;
        if dk != d.domain_key {
            return Err(conflict("Domain key drift"));
        }
        let old_enabled:Option<bool>=sqlx::query_scalar("SELECT enabled FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 FOR UPDATE").bind(d.domain_id).bind(pair.old_principal_id).bind(key).fetch_optional(&mut *tx).await?;
        let new_enabled:Option<bool>=sqlx::query_scalar("SELECT enabled FROM domain_role_bindings WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 FOR UPDATE").bind(d.domain_id).bind(pair.new_principal_id).bind(key).fetch_optional(&mut *tx).await?;
        if old_enabled != Some(true) || new_enabled == Some(true) {
            return Err(conflict(format!("Domain binding drift pair {index}")));
        }
        let disabled = sqlx::query("UPDATE domain_role_bindings SET enabled=FALSE,disabled_at=now() WHERE domain_id=$1 AND principal_id=$2 AND role_key=$3 AND enabled").bind(d.domain_id).bind(pair.old_principal_id).bind(key).execute(&mut *tx).await?;
        if disabled.rows_affected() != 1 {
            return Err(conflict(format!("domain disable CAS failed pair {index}")));
        }
        sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled,disabled_at) VALUES($1,$2,$3,$4,TRUE,NULL) ON CONFLICT(domain_id,principal_id,role_key) DO UPDATE SET enabled=TRUE,disabled_at=NULL").bind(deterministic(&format!("domain:{index}:{}:{key}",d.domain_id))).bind(d.domain_id).bind(pair.new_principal_id).bind(key).execute(&mut *tx).await?;
    }
    let rs: Vec<_> = plan
        .current_responsibility_tuples
        .iter()
        .filter(|x| {
            x.old_principal_id == pair.old_principal_id
                && x.new_principal_id == pair.new_principal_id
        })
        .collect();
    let mut migrated = 0;
    let mut skipped = 0;
    for r in rs {
        match migrate_responsibility(&mut tx, index, pair, &migration_id, r, actor).await? {
            true => migrated += 1,
            false => skipped += 1,
        }
    }
    let command = deterministic(&format!("summary-command:{index}"));
    let audit = deterministic(&format!("summary-audit:{index}"));
    let request = json!({"migrationId":migration_id,"planFileSha256":PLAN_SHA,"pairIndex":index,"oldPrincipal":pair.old_principal_id,"newPrincipal":pair.new_principal_id,"migrationActor":actor});
    let response = json!({"commandId":command,"auditId":audit,"outcome":"COMMITTED","pairIndex":index,"domains":domains.len(),"responsibilitiesMigrated":migrated,"responsibilitiesSkipped":skipped});
    insert_completed_receipt(&mut tx, command, actor, &summary_key, &request, &response).await?;
    sqlx::query("INSERT INTO workflow_security_audits(audit_id,principal_id,action,resource_type,resource_id,details) VALUES($1,$2,$3,'TRUSTED_FLEET_PRINCIPAL_CUTOVER',$4,$5)").bind(audit).bind(actor).bind(AUDIT_ACTION).bind(&migration_id).bind(&response).execute(&mut *tx).await?;
    let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM domain_role_bindings WHERE principal_id=$1 AND enabled AND domain_id=ANY($2)").bind(pair.old_principal_id).bind(&domains.iter().map(|x|x.domain_id).collect::<Vec<_>>()).fetch_one(&mut *tx).await?;
    if remaining != 0 {
        return Err(conflict("post-commit Domain reobserve failed"));
    }
    if let Err(commit_error) = tx.commit().await {
        return match pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await {
            Ok(true) => Ok("COMMITTED"),
            Ok(false) => match pair_pre_exact(pool, plan, pair, display, actor, &migration_id).await {
                Ok(true) => Err(outcome_error("ROLLED_BACK", format!("pair {index} commit was not applied: {commit_error}"))),
                Ok(false) => Err(conflict(format!("pair {index} has partial state after commit error: {commit_error}"))),
                Err(read_error) => Err(outcome_error("OUTCOME_UNKNOWN", format!("pair {index} commit error {commit_error}; preread failed: {read_error}"))),
            },
            Err(read_error) => Err(outcome_error("OUTCOME_UNKNOWN", format!("pair {index} commit error {commit_error}; terminal reread failed: {read_error}"))),
        };
    }
    if pair_terminal_exact(pool, plan, pair, display, actor, &migration_id).await? {
        Ok("COMMITTED")
    } else {
        Err(conflict(format!(
            "pair {index} committed but terminal reread disagrees"
        )))
    }
}
async fn insert_completed_receipt(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: Uuid,
    key: &str,
    request: &Value,
    response: &Value,
) -> Result<()> {
    let rh = digest(request)?;
    let rd = digest(response)?;
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status) VALUES($1,$2,$3,$4,$5,'PROCESSING')")
        .bind(id).bind(actor).bind(key).bind(COMMAND_TYPE).bind(rh)
        .execute(&mut **tx).await?;
    let updated = sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=200,response_body=$2,response_digest=$3,completed_at=now() WHERE command_id=$1 AND receipt_status='PROCESSING'")
        .bind(id).bind(response).bind(rd).execute(&mut **tx).await?;
    if updated.rows_affected() != 1 {
        return Err(conflict(
            "receipt completion did not affect exactly one row",
        ));
    }
    Ok(())
}
async fn migrate_responsibility(
    tx: &mut Transaction<'_, Postgres>,
    index: usize,
    pair: &Pair,
    migration_id: &str,
    r: &Responsibility,
    actor: Uuid,
) -> Result<bool> {
    let row=sqlx::query("SELECT wi.domain_id,wi.definition_version_id,wi.created_by_principal_id,wi.current_context_revision_id,wi.current_node_visit_id,wi.workflow_state_version,wi.cancelled,wi.archived_at,v.node_id,v.assignee_principal_id,n.node_type::text AS node_type FROM workflow_instances wi JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id WHERE wi.workflow_instance_id=$1 FOR UPDATE OF wi,v").bind(r.workflow_instance_id).fetch_one(&mut **tx).await?;
    let cancelled: bool = row.try_get("cancelled").unwrap_or(false);
    let archived: Option<chrono::DateTime<Utc>> = row.try_get("archived_at").ok();
    let terminal = row.get::<String, _>("node_type") == "TERMINAL";
    if cancelled || archived.is_some() || terminal {
        if row.get::<Uuid, _>("domain_id") != r.domain_id {
            return Err(conflict(format!(
                "terminal responsibility domain drift {}",
                r.workflow_instance_id
            )));
        }
        let source_ok: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_node_visits WHERE node_visit_id=$1 AND workflow_instance_id=$2 AND node_id=$3 AND assignee_principal_id=$4")
            .bind(r.current_visit_id).bind(r.workflow_instance_id).bind(r.node_id).bind(pair.old_principal_id)
            .fetch_one(&mut **tx).await?;
        if source_ok != 1 {
            return Err(conflict(format!(
                "terminal responsibility source drift {}",
                r.workflow_instance_id
            )));
        }
        return Ok(false);
    }
    if row.get::<Uuid, _>("domain_id") != r.domain_id
        || row.get::<Uuid, _>("current_node_visit_id") != r.current_visit_id
        || row.get::<Uuid, _>("node_id") != r.node_id
        || row.get::<Uuid, _>("assignee_principal_id") != pair.old_principal_id
        || row.get::<i32, _>("workflow_state_version") != r.expected_state_version
    {
        return Err(conflict(format!(
            "active responsibility drift {}",
            r.workflow_instance_id
        )));
    }
    let open_assistance: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_assistance_cases WHERE node_visit_id=$1 AND status IN ('OWNER_PENDING','HUMAN_REQUIRED')")
        .bind(r.current_visit_id).fetch_one(&mut **tx).await?;
    if open_assistance != 0 {
        return Err(conflict(format!(
            "open assistance blocks responsibility {}",
            r.workflow_instance_id
        )));
    }
    let context: Option<Uuid> = row.try_get("current_context_revision_id").ok();
    let before = BeforeSnapshotV1::new(
        r.workflow_instance_id,
        r.domain_id,
        row.get("definition_version_id"),
        row.get("created_by_principal_id"),
        &WorkflowProjection {
            current_context_revision_id: context,
            current_node_visit_id: Some(r.current_visit_id),
            workflow_state_version: r.expected_state_version,
        },
    )
    .digest()
    .map_err(|e| conflict(e.to_string()))?;
    let target = deterministic(&format!("visit:{index}:{}", r.workflow_instance_id));
    let next:i32=sqlx::query_scalar("SELECT COALESCE(MAX(visit_number),0)+1 FROM workflow_node_visits WHERE workflow_instance_id=$1 AND node_id=$2").bind(r.workflow_instance_id).bind(r.node_id).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO workflow_node_visits(node_visit_id,workflow_instance_id,node_id,visit_number,assignee_principal_id,entered_by_transition_id) VALUES($1,$2,$3,$4,$5,NULL)").bind(target).bind(r.workflow_instance_id).bind(r.node_id).bind(next).bind(pair.new_principal_id).execute(&mut **tx).await?;
    let new_version = r.expected_state_version + 1;
    let cas = sqlx::query("UPDATE workflow_instances SET current_node_visit_id=$1,workflow_state_version=$2,updated_at=now() WHERE workflow_instance_id=$3 AND current_node_visit_id=$4 AND workflow_state_version=$5").bind(target).bind(new_version).bind(r.workflow_instance_id).bind(r.current_visit_id).bind(r.expected_state_version).execute(&mut **tx).await?;
    if cas.rows_affected() != 1 {
        return Err(conflict(format!(
            "responsibility CAS failed for {}",
            r.workflow_instance_id
        )));
    }
    let after = BeforeSnapshotV1::new(
        r.workflow_instance_id,
        r.domain_id,
        row.get("definition_version_id"),
        row.get("created_by_principal_id"),
        &WorkflowProjection {
            current_context_revision_id: context,
            current_node_visit_id: Some(target),
            workflow_state_version: new_version,
        },
    )
    .digest()
    .map_err(|e| conflict(e.to_string()))?;
    let command = deterministic(&format!("command:{index}:{}", r.workflow_instance_id));
    let event = deterministic(&format!("event:{index}:{}", r.workflow_instance_id));
    let occurred = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
    let data = json!({"migration_id":migration_id,"plan_sha256":PLAN_SHA,"pair_index":index,"old_agent_id":pair.old_agent_id,"new_agent_id":pair.new_agent_id,"old_principal_id":pair.old_principal_id,"new_principal_id":pair.new_principal_id,"workflow_instance_id":r.workflow_instance_id,"old_visit_id":r.current_visit_id,"new_visit_id":target,"node_id":r.node_id,"expected_state_version":r.expected_state_version,"resulting_state_version":new_version,"before_projection_digest":before,"after_projection_digest":after,"causation_id":Value::Null,"correlation_id":Value::Null,"occurred_at":occurred});
    let request = json!({"migrationId":migration_id,"planFileSha256":PLAN_SHA,"pairIndex":index,"workflowInstanceId":r.workflow_instance_id,"migrationActor":actor});
    let response = json!({"commandId":command,"eventId":event,"outcome":"COMMITTED","beforeProjectionDigest":before,"afterProjectionDigest":after});
    insert_completed_receipt(
        tx,
        command,
        actor,
        &format!("{migration_id}:{}", r.workflow_instance_id),
        &request,
        &response,
    )
    .await?;
    let ed = digest(&data)?;
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,source_node_visit_id,target_node_visit_id,context_revision_id,event_data,event_data_digest,actor_principal_id,from_node_id,to_node_id,old_workflow_state_version,new_workflow_state_version) VALUES($1,$2,$3,'v1',$4,$5,$6,$7,$8,$9,$10,$11,$12,$12,$13,$3)").bind(event).bind(r.workflow_instance_id).bind(new_version).bind(command).bind(EVENT_TYPE).bind(r.current_visit_id).bind(target).bind(context).bind(&data).bind(ed).bind(actor).bind(r.node_id).bind(r.expected_state_version).execute(&mut **tx).await?;
    Ok(true)
}
