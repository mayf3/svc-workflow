use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::workflow_instance::recovery::{
    BeforeSnapshotV1, RecoveryError, WorkflowProjection,
};

use super::event_fields::{
    admin_payload_is_bounded, event_data, exact_keys, optional_string_field, string_field,
    uuid_field,
};
use super::import_event;
use super::rows::{ContextFact, EventFact, InstanceRow, SubmissionFact, TransitionFact, VisitFact};

const TRUSTED_FLEET_PLAN_SHA256: &str =
    "0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606";
const TRUSTED_FLEET_PAIRS: [(&str, &str, &str, &str); 86] = [
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

fn invalid(detail: impl Into<String>) -> RecoveryError {
    RecoveryError::InvalidImmutableFacts(detail.into())
}

fn trusted_fleet_migration_id(
    pair_index: usize,
    pair: (&str, &str, &str, &str),
) -> Result<String, RecoveryError> {
    let _ = pair_index;
    let value = serde_json::json!({
        "newPrincipal": pair.3,
        "oldPrincipal": pair.2,
        "planFileSha256": TRUSTED_FLEET_PLAN_SHA256,
    });
    let canonical = jcs_canonicalize::canonicalize(&value.to_string())
        .map_err(|error| RecoveryError::StorageError(error.to_string()))?;
    Ok(format!(
        "trusted-fleet-v1:{}",
        hex::encode(Sha256::digest(canonical.as_bytes()))
    ))
}

struct Replay<'a> {
    instance: &'a InstanceRow,
    definition_digest: Option<&'a str>,
    contexts: HashMap<Uuid, &'a ContextFact>,
    visits: HashMap<Uuid, &'a VisitFact>,
    submissions: HashMap<Uuid, &'a SubmissionFact>,
    transitions: HashMap<Uuid, &'a TransitionFact>,
    introduced_contexts: HashSet<Uuid>,
    introduced_visits: HashSet<Uuid>,
    introduced_submissions: HashSet<Uuid>,
    visit_counts: HashMap<Uuid, i32>,
    current_context: Option<Uuid>,
    current_visit: Option<Uuid>,
    version: i32,
    /// Per-case replayed state, keyed by `assistanceCaseId`. Tracks the case's
    /// current status and the node visit it is bound to, reconstructed purely
    /// from the assistance event stream. Used to reject histories the runtime
    /// could never produce: duplicate request/escalate, resolve of an unknown
    /// case, cross-visit case串线, post-resolution events, and transitions
    /// attempted while the source visit still has an open case.
    assistance_cases: HashMap<Uuid, (String, Uuid)>,
}

impl<'a> Replay<'a> {
    fn new(
        instance: &'a InstanceRow,
        definition_digest: Option<&'a str>,
        contexts: &'a [ContextFact],
        visits: &'a [VisitFact],
        submissions: &'a [SubmissionFact],
        transitions: &'a [TransitionFact],
    ) -> Self {
        Self {
            instance,
            definition_digest,
            contexts: contexts
                .iter()
                .map(|fact| (fact.context_revision_id, fact))
                .collect(),
            visits: visits
                .iter()
                .map(|fact| (fact.node_visit_id, fact))
                .collect(),
            submissions: submissions
                .iter()
                .map(|fact| (fact.submission_id, fact))
                .collect(),
            transitions: transitions
                .iter()
                .map(|fact| (fact.transition_id, fact))
                .collect(),
            introduced_contexts: HashSet::new(),
            introduced_visits: HashSet::new(),
            introduced_submissions: HashSet::new(),
            visit_counts: HashMap::new(),
            current_context: None,
            current_visit: None,
            version: 0,
            assistance_cases: HashMap::new(),
        }
    }

    fn visit(&self, id: Option<Uuid>) -> Result<&'a VisitFact, RecoveryError> {
        id.and_then(|value| self.visits.get(&value).copied())
            .ok_or_else(|| invalid("event references an invalid node visit"))
    }

    /// True if the replayed assistance state has an open case (`OWNER_PENDING`
    /// or `HUMAN_REQUIRED`) bound to `visit_id`. Mirrors the runtime transition
    /// gate (`has_open_assistance`) so replay rejects transition events the
    /// runtime would have refused with `AssistanceOpen`.
    fn visit_has_open_assistance(&self, visit_id: Uuid) -> bool {
        self.assistance_cases.values().any(|(status, visit)| {
            *visit == visit_id && (status == "OWNER_PENDING" || status == "HUMAN_REQUIRED")
        })
    }

    fn context(&self, id: Option<Uuid>) -> Result<&'a ContextFact, RecoveryError> {
        id.and_then(|value| self.contexts.get(&value).copied())
            .ok_or_else(|| invalid("event references an invalid context revision"))
    }

    fn introduce_visit(&mut self, visit: &VisitFact) -> Result<(), RecoveryError> {
        if !self.introduced_visits.insert(visit.node_visit_id) {
            return Err(invalid("node visit is introduced more than once"));
        }
        let count = self.visit_counts.entry(visit.node_id).or_insert(0);
        *count = count
            .checked_add(1)
            .ok_or_else(|| invalid("node visit count overflow"))?;
        if visit.visit_number != *count {
            return Err(invalid("node visit number does not follow event order"));
        }
        Ok(())
    }

    fn introduce_context(&mut self, context: &ContextFact) -> Result<(), RecoveryError> {
        if !self.introduced_contexts.insert(context.context_revision_id) {
            return Err(invalid("context revision is introduced more than once"));
        }
        Ok(())
    }

    fn introduce_submission(&mut self, submission: &SubmissionFact) -> Result<(), RecoveryError> {
        if !self.introduced_submissions.insert(submission.submission_id) {
            return Err(invalid("submission is introduced more than once"));
        }
        Ok(())
    }

    fn validate_node_columns(
        &self,
        event: &EventFact,
        source: Option<&VisitFact>,
        target: Option<&VisitFact>,
    ) -> Result<(), RecoveryError> {
        if event.from_node_id != source.map(|visit| visit.node_id)
            || event.to_node_id != target.map(|visit| visit.node_id)
        {
            return Err(invalid("event node fields disagree with visit references"));
        }
        Ok(())
    }

    fn apply_initial(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let imported = event.event_type == "WORKFLOW_INSTANCE_IMPORTED";
        let keys = if imported {
            [
                "legacySystem",
                "legacyRecordId",
                "legacySnapshotDigest",
                "importedNodeId",
                "importedAt",
                "creatorResolution",
            ]
            .as_slice()
        } else {
            [
                "definition_version_id",
                "definition_digest",
                "initial_node_id",
                "assignee_resolution_type",
            ]
            .as_slice()
        };
        let context = self.context(event.context_revision_id)?;
        let visit = self.visit(event.target_node_visit_id)?;
        if self.version != 0
            || !exact_keys(data, keys)
            || event.source_node_visit_id.is_some()
            || event.submission_id.is_some()
            || event.transition_effect.is_some()
            || event.from_node_id.is_some()
            || event.to_node_id.is_some()
            || context.revision_number != 1
            || context.previous_revision_id.is_some()
            || visit.entered_by_transition_id.is_some()
            || visit.visit_number != 1
        {
            return Err(invalid(
                "initial event does not introduce revision 1 and visit 1",
            ));
        }
        if imported {
            import_event::validate(data, event, self.instance, context, visit)?;
        } else if uuid_field(data, "definition_version_id")
            != Some(self.instance.definition_version_id)
            || string_field(data, "definition_digest") != Some(self.definition_digest.unwrap_or(""))
            || uuid_field(data, "initial_node_id") != Some(visit.node_id)
            || string_field(data, "assignee_resolution_type") != visit.assignee_ref_type.as_deref()
            || visit.node_type != "DRAFT"
            || visit.assignee_principal_id.is_none()
        {
            return Err(invalid(
                "creation event data does not match the initial facts",
            ));
        }
        self.introduce_context(context)?;
        self.introduce_visit(visit)?;
        self.current_context = Some(context.context_revision_id);
        self.current_visit = Some(visit.node_visit_id);
        Ok(())
    }

    fn apply_context_revision(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let context = self.context(event.context_revision_id)?;
        let visit = self.visit(event.source_node_visit_id)?;
        let previous = self.context(self.current_context)?;
        if !exact_keys(
            data,
            &[
                "previous_context_revision_id",
                "new_context_revision_id",
                "previous_payload_digest",
                "new_payload_digest",
                "current_node_id",
            ],
        ) || event.source_node_visit_id != self.current_visit
            || event.target_node_visit_id != self.current_visit
            || event.submission_id.is_some()
            || event.transition_effect.is_some()
            || event.from_node_id.is_some()
            || event.to_node_id.is_some()
            || context.previous_revision_id != self.current_context
            || context.revision_number != previous.revision_number + 1
            || uuid_field(data, "previous_context_revision_id") != self.current_context
            || uuid_field(data, "new_context_revision_id") != Some(context.context_revision_id)
            || string_field(data, "previous_payload_digest")
                != Some(previous.payload_digest.as_str())
            || string_field(data, "new_payload_digest") != Some(context.payload_digest.as_str())
            || uuid_field(data, "current_node_id") != Some(visit.node_id)
            || visit.node_type != "DRAFT"
        {
            return Err(invalid(
                "context revision event disagrees with replay state",
            ));
        }
        self.introduce_context(context)?;
        self.current_context = Some(context.context_revision_id);
        Ok(())
    }

    fn transition_and_visits(
        &self,
        event: &EventFact,
        data: &serde_json::Value,
    ) -> Result<(&'a VisitFact, &'a VisitFact, &'a TransitionFact), RecoveryError> {
        let source = self.visit(event.source_node_visit_id)?;
        let target = self.visit(event.target_node_visit_id)?;
        let transition_id = uuid_field(data, "transition_definition_id")
            .ok_or_else(|| invalid("transition event has no valid transition id"))?;
        let transition = self
            .transitions
            .get(&transition_id)
            .copied()
            .ok_or_else(|| invalid("transition event references an invalid definition"))?;
        if event.source_node_visit_id != self.current_visit
            || self.introduced_visits.contains(&target.node_visit_id)
            || target.entered_by_transition_id != Some(transition.transition_id)
            || transition.source_node_id != source.node_id
            || transition.target_node_id != target.node_id
            || event.transition_effect.as_deref() != Some(transition.transition_effect.as_str())
            || string_field(data, "transition_key") != Some(transition.transition_key.as_str())
            || string_field(data, "transition_effect")
                != Some(transition.transition_effect.as_str())
            || uuid_field(data, "source_node_id") != Some(source.node_id)
            || uuid_field(data, "target_node_id") != Some(target.node_id)
            || uuid_field(data, "source_node_visit_id") != Some(source.node_visit_id)
            || uuid_field(data, "target_node_visit_id") != Some(target.node_visit_id)
        {
            return Err(invalid(
                "transition event disagrees with definition or replay state",
            ));
        }
        self.validate_node_columns(event, Some(source), Some(target))?;
        Ok((source, target, transition))
    }

    fn apply_transition(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        if !exact_keys(
            data,
            &[
                "transition_definition_id",
                "transition_key",
                "transition_effect",
                "source_node_id",
                "target_node_id",
                "source_node_visit_id",
                "target_node_visit_id",
                "context_revision_id",
                "submission_payload_digest",
            ],
        ) || event.context_revision_id != self.current_context
            || uuid_field(data, "context_revision_id") != self.current_context
        {
            return Err(invalid("transition event data shape is invalid"));
        }
        // Reproduce the runtime transition gate: a visit with an open
        // assistance case cannot be advanced. A legitimate event log can never
        // contain a transition whose source visit still has an open case, so a
        // hit here proves the history is forged/corrupted.
        if let Some(source_visit) = self.current_visit {
            if self.visit_has_open_assistance(source_visit) {
                return Err(invalid(
                    "transition is blocked by an open assistance case on the source visit",
                ));
            }
        }
        let (_, target, transition) = self.transition_and_visits(event, data)?;
        match event.submission_id {
            Some(id) => {
                let submission =
                    self.submissions.get(&id).copied().ok_or_else(|| {
                        invalid("transition event references an invalid submission")
                    })?;
                if submission.source_node_visit_id != event.source_node_visit_id.unwrap()
                    || Some(submission.context_revision_id) != self.current_context
                    || submission.transition_id != transition.transition_id
                    || optional_string_field(data, "submission_payload_digest")
                        != Some(Some(submission.payload_digest.as_str()))
                {
                    return Err(invalid("transition submission disagrees with event"));
                }
                self.introduce_submission(submission)?;
            }
            None if optional_string_field(data, "submission_payload_digest") != Some(None) => {
                return Err(invalid(
                    "transition event omits a referenced submission digest",
                ));
            }
            None => {}
        }
        self.introduce_visit(target)?;
        self.current_visit = Some(target.node_visit_id);
        Ok(())
    }

    fn apply_combined(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let keys = [
            "previous_context_revision_id",
            "new_context_revision_id",
            "previous_context_payload_digest",
            "new_context_payload_digest",
            "transition_definition_id",
            "transition_key",
            "transition_effect",
            "source_node_id",
            "target_node_id",
            "source_node_visit_id",
            "target_node_visit_id",
            "submission_payload_digest",
        ];
        let previous = self.context(self.current_context)?;
        let context = self.context(event.context_revision_id)?;
        if !exact_keys(data, &keys)
            || event.transition_effect.as_deref() != Some("ADVANCE")
            || context.previous_revision_id != self.current_context
            || context.revision_number != previous.revision_number + 1
            || uuid_field(data, "previous_context_revision_id") != self.current_context
            || uuid_field(data, "new_context_revision_id") != Some(context.context_revision_id)
            || string_field(data, "previous_context_payload_digest")
                != Some(previous.payload_digest.as_str())
            || string_field(data, "new_context_payload_digest")
                != Some(context.payload_digest.as_str())
        {
            return Err(invalid(
                "combined event context disagrees with replay state",
            ));
        }
        // Same open-assistance transition gate as a plain transition
        // (REVISE_CONTEXT_AND_TRANSITION runs the same `has_open_assistance`
        // check at runtime). Admin emergency override is intentionally NOT
        // gated — it legitimately voids open cases.
        if let Some(source_visit) = self.current_visit {
            if self.visit_has_open_assistance(source_visit) {
                return Err(invalid(
                    "combined transition is blocked by an open assistance case on the source visit",
                ));
            }
        }
        let (_, target, transition) = self.transition_and_visits(event, data)?;
        let submission = event
            .submission_id
            .and_then(|id| self.submissions.get(&id).copied())
            .ok_or_else(|| invalid("combined event requires a submission"))?;
        if submission.source_node_visit_id != event.source_node_visit_id.unwrap()
            || submission.context_revision_id != context.context_revision_id
            || submission.transition_id != transition.transition_id
            || string_field(data, "submission_payload_digest")
                != Some(submission.payload_digest.as_str())
        {
            return Err(invalid("combined submission disagrees with event"));
        }
        self.introduce_context(context)?;
        self.introduce_submission(submission)?;
        self.introduce_visit(target)?;
        self.current_context = Some(context.context_revision_id);
        self.current_visit = Some(target.node_visit_id);
        Ok(())
    }

    fn apply_admin(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let source = self.visit(event.source_node_visit_id)?;
        let target = self.visit(event.target_node_visit_id)?;
        let operation = string_field(data, "operation");
        let expected_effect = match operation {
            Some("MOVE_TO_NODE") if target.node_type != "TERMINAL" => "ADVANCE",
            Some("TERMINATE_INSTANCE") if target.node_type == "TERMINAL" => "TERMINATE",
            _ => return Err(invalid("admin operation does not match target node type")),
        };
        let before = BeforeSnapshotV1::new(
            self.instance.workflow_instance_id,
            self.instance.domain_id,
            self.instance.definition_version_id,
            self.instance.created_by_principal_id,
            &WorkflowProjection {
                current_context_revision_id: self.current_context,
                current_node_visit_id: self.current_visit,
                workflow_state_version: self.version,
            },
        )
        .digest()?;
        if !exact_keys(
            data,
            &[
                "operation",
                "reason",
                "relatedReferences",
                "beforeSnapshotDigest",
            ],
        ) || event.source_node_visit_id != self.current_visit
            || event.context_revision_id != self.current_context
            || event.submission_id.is_some()
            || self.introduced_visits.contains(&target.node_visit_id)
            || target.entered_by_transition_id.is_some()
            || event.transition_effect.as_deref() != Some(expected_effect)
            || !admin_payload_is_bounded(data)
            || string_field(data, "beforeSnapshotDigest") != Some(before.as_str())
        {
            return Err(invalid("admin event disagrees with replay state"));
        }
        self.validate_node_columns(event, Some(source), Some(target))?;
        self.introduce_visit(target)?;
        self.current_visit = Some(target.node_visit_id);
        Ok(())
    }

    fn apply_assistance(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let visit = self.visit(event.source_node_visit_id)?;
        let case_id = uuid_field(data, "assistanceCaseId")
            .ok_or_else(|| invalid("assistance event has no valid assistanceCaseId"))?;
        let previous_status = optional_string_field(data, "previousStatus");
        let new_status = string_field(data, "newStatus");
        let status_shape_valid = match event.event_type.as_str() {
            "ASSISTANCE_REQUESTED" => {
                previous_status == Some(None) && new_status == Some("OWNER_PENDING")
            }
            "ASSISTANCE_ESCALATED_TO_HUMAN" => {
                previous_status == Some(Some("OWNER_PENDING"))
                    && new_status == Some("HUMAN_REQUIRED")
            }
            "ASSISTANCE_RESOLVED" => {
                matches!(
                    previous_status,
                    Some(Some("OWNER_PENDING" | "HUMAN_REQUIRED"))
                ) && new_status == Some("RESOLVED")
            }
            _ => false,
        };
        let payload_digest = string_field(data, "payloadDigest");
        if !exact_keys(
            data,
            &[
                "assistanceCaseId",
                "previousStatus",
                "newStatus",
                "payloadDigest",
            ],
        ) || !status_shape_valid
            || payload_digest.is_none_or(|value| {
                value.len() != 64
                    || !value
                        .as_bytes()
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
            })
            || event.source_node_visit_id != self.current_visit
            || event.target_node_visit_id != self.current_visit
            || event.context_revision_id != self.current_context
            || event.submission_id.is_some()
            || event.transition_effect.is_some()
        {
            return Err(invalid("assistance event disagrees with replay state"));
        }
        self.validate_node_columns(event, Some(visit), Some(visit))?;

        // Cross-event state machine. The runtime binds an assistance case to a
        // single visit for its whole life, and the transition gate refuses to
        // advance a visit that still has an open case — so the current visit
        // cannot move while a case on it is open. Replay reconstructs that here
        // and rejects anything the live system could not have emitted.
        let current_visit = self
            .current_visit
            .ok_or_else(|| invalid("assistance event precedes the initial visit"))?;
        match event.event_type.as_str() {
            "ASSISTANCE_REQUESTED" => {
                if self.assistance_cases.contains_key(&case_id) {
                    return Err(invalid("assistance case is requested more than once"));
                }
                self.assistance_cases
                    .insert(case_id, ("OWNER_PENDING".to_string(), current_visit));
            }
            "ASSISTANCE_ESCALATED_TO_HUMAN" => {
                let (ref_status, ref_visit) = self
                    .assistance_cases
                    .get(&case_id)
                    .map(|(status, visit)| (status.clone(), *visit))
                    .ok_or_else(|| invalid("assistance escalation references an unknown case"))?;
                if ref_visit != current_visit {
                    return Err(invalid(
                        "assistance case is escalated on a different visit than it was opened",
                    ));
                }
                if ref_status != "OWNER_PENDING" {
                    return Err(invalid(
                        "assistance case is not OWNER_PENDING when it is escalated",
                    ));
                }
                self.assistance_cases
                    .insert(case_id, ("HUMAN_REQUIRED".to_string(), current_visit));
            }
            "ASSISTANCE_RESOLVED" => {
                let (ref_status, ref_visit) = self
                    .assistance_cases
                    .get(&case_id)
                    .map(|(status, visit)| (status.clone(), *visit))
                    .ok_or_else(|| invalid("assistance resolution references an unknown case"))?;
                if ref_visit != current_visit {
                    return Err(invalid(
                        "assistance case is resolved on a different visit than it was opened",
                    ));
                }
                if ref_status != "OWNER_PENDING" && ref_status != "HUMAN_REQUIRED" {
                    return Err(invalid(
                        "assistance case is resolved after it already reached a terminal state",
                    ));
                }
                if previous_status != Some(Some(ref_status.as_str())) {
                    return Err(invalid(
                        "assistance event previousStatus disagrees with replayed case status",
                    ));
                }
                self.assistance_cases
                    .insert(case_id, ("RESOLVED".to_string(), current_visit));
            }
            _ => return Err(invalid("unsupported assistance event type")),
        }
        Ok(())
    }

    fn apply_trusted_fleet_successor(&mut self, event: &EventFact) -> Result<(), RecoveryError> {
        let data = event_data(event)?;
        let keys = [
            "migration_id",
            "plan_sha256",
            "pair_index",
            "old_agent_id",
            "new_agent_id",
            "old_principal_id",
            "new_principal_id",
            "workflow_instance_id",
            "old_visit_id",
            "new_visit_id",
            "node_id",
            "expected_state_version",
            "resulting_state_version",
            "before_projection_digest",
            "after_projection_digest",
            "causation_id",
            "correlation_id",
            "occurred_at",
        ];
        if !exact_keys(data, &keys)
            || string_field(data, "plan_sha256") != Some(TRUSTED_FLEET_PLAN_SHA256)
        {
            return Err(invalid(
                "trusted fleet successor event shape/plan is invalid",
            ));
        }
        let pair_index = data
            .get("pair_index")
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| (1..=TRUSTED_FLEET_PAIRS.len()).contains(v))
            .ok_or_else(|| invalid("trusted fleet successor pair_index is invalid"))?;
        let pair = TRUSTED_FLEET_PAIRS[pair_index - 1];
        let migration_id = trusted_fleet_migration_id(pair_index, pair)?;
        if string_field(data, "migration_id") != Some(migration_id.as_str()) {
            return Err(invalid("trusted fleet successor migration_id is invalid"));
        }
        let old_principal = Uuid::parse_str(pair.2).expect("compiled OLD UUID");
        let new_principal = Uuid::parse_str(pair.3).expect("compiled NEW UUID");
        let source = self.visit(event.source_node_visit_id)?;
        let target = self.visit(event.target_node_visit_id)?;
        let expected_version = data.get("expected_state_version").and_then(|v| v.as_i64());
        let resulting_version = data.get("resulting_state_version").and_then(|v| v.as_i64());
        let before = BeforeSnapshotV1::new(
            self.instance.workflow_instance_id,
            self.instance.domain_id,
            self.instance.definition_version_id,
            self.instance.created_by_principal_id,
            &WorkflowProjection {
                current_context_revision_id: self.current_context,
                current_node_visit_id: self.current_visit,
                workflow_state_version: self.version,
            },
        )
        .digest()?;
        if string_field(data, "old_agent_id") != Some(pair.0)
            || string_field(data, "new_agent_id") != Some(pair.1)
            || uuid_field(data, "old_principal_id") != Some(old_principal)
            || uuid_field(data, "new_principal_id") != Some(new_principal)
            || uuid_field(data, "workflow_instance_id") != Some(self.instance.workflow_instance_id)
            || uuid_field(data, "old_visit_id") != Some(source.node_visit_id)
            || uuid_field(data, "new_visit_id") != Some(target.node_visit_id)
            || uuid_field(data, "node_id") != Some(source.node_id)
            || expected_version != Some(i64::from(self.version))
            || resulting_version != Some(i64::from(self.version + 1))
            || string_field(data, "before_projection_digest") != Some(before.as_str())
            || data.get("causation_id") != Some(&serde_json::Value::Null)
            || data.get("correlation_id") != Some(&serde_json::Value::Null)
            || string_field(data, "occurred_at")
                .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                .is_none()
            || event.command_id.is_none()
            || event.actor_principal_id == old_principal
            || event.actor_principal_id == new_principal
            || event.source_node_visit_id != self.current_visit
            || event.context_revision_id != self.current_context
            || event.submission_id.is_some()
            || event.transition_effect.is_some()
            || self.introduced_visits.contains(&target.node_visit_id)
            || target.entered_by_transition_id.is_some()
            || source.node_id != target.node_id
            || source.assignee_principal_id != Some(old_principal)
            || target.assignee_principal_id != Some(new_principal)
        {
            return Err(invalid(
                "trusted fleet successor event disagrees with replay state",
            ));
        }
        self.validate_node_columns(event, Some(source), Some(target))?;
        let after = BeforeSnapshotV1::new(
            self.instance.workflow_instance_id,
            self.instance.domain_id,
            self.instance.definition_version_id,
            self.instance.created_by_principal_id,
            &WorkflowProjection {
                current_context_revision_id: self.current_context,
                current_node_visit_id: Some(target.node_visit_id),
                workflow_state_version: self.version + 1,
            },
        )
        .digest()?;
        if string_field(data, "after_projection_digest") != Some(after.as_str()) {
            return Err(invalid(
                "trusted fleet successor after projection digest mismatch",
            ));
        }
        self.introduce_visit(target)?;
        self.current_visit = Some(target.node_visit_id);
        Ok(())
    }

    fn apply(&mut self, event: &EventFact, expected_sequence: i32) -> Result<(), RecoveryError> {
        if event.workflow_instance_id != self.instance.workflow_instance_id
            || event.event_sequence != expected_sequence
            || event.event_schema_version != "v1"
            || event.old_workflow_state_version != self.version
            || event.new_workflow_state_version != expected_sequence
        {
            return Err(invalid(
                "event sequence and state versions are not contiguous",
            ));
        }
        match event.event_type.as_str() {
            "INSTANCE_CREATED" | "WORKFLOW_INSTANCE_CREATED" | "WORKFLOW_INSTANCE_IMPORTED" => {
                self.apply_initial(event)?
            }
            "CONTEXT_REVISED" | "WORKFLOW_CONTEXT_REVISED" => self.apply_context_revision(event)?,
            "WORKFLOW_TRANSITION_COMMITTED" => self.apply_transition(event)?,
            "WORKFLOW_CONTEXT_REVISED_AND_TRANSITION_COMMITTED" => self.apply_combined(event)?,
            "ADMIN_EMERGENCY_OVERRIDE_COMMITTED" => self.apply_admin(event)?,
            "PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED" => {
                self.apply_trusted_fleet_successor(event)?
            }
            "ASSISTANCE_REQUESTED" | "ASSISTANCE_ESCALATED_TO_HUMAN" | "ASSISTANCE_RESOLVED" => {
                self.apply_assistance(event)?
            }
            _ => return Err(invalid("event type is not supported by recovery replay")),
        }
        self.version = expected_sequence;
        Ok(())
    }

    fn finish(self) -> Result<WorkflowProjection, RecoveryError> {
        if self.introduced_contexts.len() != self.contexts.len()
            || self.introduced_visits.len() != self.visits.len()
            || self.introduced_submissions.len() != self.submissions.len()
        {
            return Err(invalid(
                "immutable fact exists outside its introducing event",
            ));
        }
        Ok(WorkflowProjection {
            current_context_revision_id: self.current_context,
            current_node_visit_id: self.current_visit,
            workflow_state_version: self.version,
        })
    }
}

#[cfg(test)]
mod trusted_fleet_successor_tests {
    use super::*;

    #[test]
    fn exact_pair_allowlist_has_unique_deterministic_migration_ids() {
        assert_eq!(TRUSTED_FLEET_PAIRS.len(), 86);
        let mut pairs = HashSet::new();
        let mut migrations = HashSet::new();
        for (index, pair) in TRUSTED_FLEET_PAIRS.iter().copied().enumerate() {
            Uuid::parse_str(pair.2).expect("OLD UUID");
            Uuid::parse_str(pair.3).expect("NEW UUID");
            assert!(pairs.insert((pair.0, pair.1, pair.2, pair.3)));
            let migration = trusted_fleet_migration_id(index + 1, pair).expect("migration id");
            assert!(migration.starts_with("trusted-fleet-v1:"));
            assert!(migrations.insert(migration));
        }
    }
}

pub(super) fn replay(
    instance: &InstanceRow,
    definition_digest: Option<&str>,
    contexts: &[ContextFact],
    visits: &[VisitFact],
    submissions: &[SubmissionFact],
    transitions: &[TransitionFact],
    events: &[EventFact],
) -> Result<WorkflowProjection, RecoveryError> {
    if events.is_empty() {
        return Err(invalid("event fact sequence is empty"));
    }
    let mut replay = Replay::new(
        instance,
        definition_digest,
        contexts,
        visits,
        submissions,
        transitions,
    );
    for (index, event) in events.iter().enumerate() {
        let expected = i32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| invalid("event sequence overflows i32"))?;
        replay.apply(event, expected)?;
    }
    replay.finish()
}
