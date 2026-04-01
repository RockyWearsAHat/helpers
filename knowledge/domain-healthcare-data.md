# Healthcare Data Standards: HL7 FHIR, DICOM, SNOMED, Interoperability & Compliance

## Overview

Healthcare systems interoperate via standardized data formats and exchange protocols. HL7 FHIR (Fast Healthcare Interoperability Resources) modernizes clinical data exchange with REST APIs. DICOM standardizes medical imaging. Coding systems (ICD-10, SNOMED CT) enable semantic interoperability. HIPAA/HITECH mandate privacy and security. Understanding these standards is essential for building health IT systems, EHR integrations, and health platforms.

## HL7 FHIR (Fast Healthcare Interoperability Resources)

### What FHIR Is

FHIR is a modern standard for exchanging healthcare data using REST APIs and JSON. It replaces earlier HL7 v2 (a text-based, error-prone standard) with a resource-based model and RESTful architecture.

**Published by:** HL7 International (nonprofit standards body)

**Version:** FHIR R4 (released 2019, stable). R5 (newer) in development.

**Licensing:** Open source; freely implementable.

### Core Concept: Resources

FHIR organizes healthcare data into **resources** (JSON objects). Each resource represents a clinical concept:

| Resource | Represents |
|----------|------------|
| **Patient** | Demographics, identifiers, contact info |
| **Observation** | Lab result, vital signs, physical exam finding |
| **Medication** | Drug information (drug name, strength, form) |
| **MedicationRequest** | Prescription or medication order |
| **Encounter** | Clinical visit (outpatient, inpatient, ER, etc.) |
| **Condition** | Patient problem or diagnosis |
| **Procedure** | Surgical or therapeutic procedure performed |
| **DiagnosticReport** | Aggregated test result (bundled observations) |
| **DocumentReference** | Reference to external document (PDF note, image) |

Each resource has:
- **Elements** (data fields): id, status, coding, text, references to other resources
- **Cardinality:** 0..1 (optional, at most one), 1..1 (required), 0..* (optional, many), 1..* (required, at least one)
- **Human-readable** (`text` element with narrative) + machine-readable (`code`, `coding` arrays)

### REST API Model

FHIR exposes resources over HTTP REST:

```
GET /Patient/12345
Response:
{
  "resourceType": "Patient",
  "id": "12345",
  "identifier": [{
    "system": "urn:oid:1.2.840.113619.6.2.1",
    "value": "MRN123456"
  }],
  "name": [{"given": ["John"], "family": "Doe"}],
  "birthDate": "1980-01-15",
  "contact": [{
    "relationship": [{"coding": [{"system": "http://terminology.hl7.org/CodeSystem/v2-0131", "code": "N"}]}],
    "name": {"text": "Jane Doe"},
    "telecom": [{"system": "phone", "value": "(555) 123-4567"}]
  }]
}

GET /Patient/12345/Observation
Response (all observations for patient 12345):
[
  {"resourceType": "Observation", "id": "obs1", "code": {...}, "valueQuantity": {...}},
  {"resourceType": "Observation", "id": "obs2", ...},
  ...
]

POST /Patient
Body:
{
  "resourceType": "Patient",
  "name": [{"given": ["Jane"], "family": "Smith"}],
  ...
}
Response: 201 Created, Location: /Patient/67890
```

### Coding & Terminology Bindings

Codes enable semantic interoperability. A resource element can be bound to a coding system:

```
"code": {
  "coding": [
    {
      "system": "http://loinc.org",
      "code": "2085-9",
      "display": "Cholesterol [Mass/volume] in Serum or Plasma"
    }
  ],
  "text": "Cholesterol"
}
```

Multiple codings allowed (e.g., to map between systems).

**Common coding systems:**
- **LOINC** (Logical Observation Identifiers Names & Codes): Lab, imaging, vital signs
- **SNOMED CT** (Systematized Nomenclature of Medicine Clinical Terms): Diagnoses, procedures, findings
- **ICD-10-CM** (International Classification of Diseases): Diagnoses (billing)
- **CVX** (CDC Vaccine Codes): Immunizations
- **CPT** (Current Procedural Terminology): Procedures (billing)

### References & Relationships

Resources link to other resources:

```
MedicationRequest → Medication (references)
MedicationRequest → Patient (who it's for)
Observation → Patient (who has the result)
Observation → Performer (who ordered/performed)
DiagnosticReport → Observation[] (bundles observations)
Encounter → Participant[] (who was involved)
```

References are URLs:
```
"subject": {"reference": "Patient/12345"}
"performer": [{"reference": "Practitioner/doc1", "display": "Dr. Smith"}]
```

This enables querying related resources: "Get all observations for this patient" or "Get all encounters where this provider participated."

## DICOM (Digital Imaging and Communications in Medicine)

### Overview

DICOM standardizes medical imaging (X-rays, CT, MRI, ultrasound, etc.), enabling interoperability between imaging devices (scanners), servers (PACS: Picture Archiving and Communication Systems), and viewers (radiologist workstations).

**Published by:** NEMA (National Electrical Manufacturers Association)

**Version:** DICOM 2024

### DICOM File Structure

A DICOM file contains:
- **Header (metadata):** Patient name, ID, scan date, modality (CT, MRI, etc.), acquisition parameters
- **Pixel data:** Raw image bytes (can be compressed: JPEG, JPEG-LS, JPEG2000, RLE)

```
Binary DICOM file layout:
[128 bytes preamble]
[DICM signature]
[File Meta Information Group (metadata tags)]
[Dataset (image + clinical attributes)]
```

### DICOM Tags (Attributes)

Data organized as (Group, Element) tags:

| Tag | Name | Example Value |
|-----|------|----------------|
| (0x0010, 0x0010) | Patient Name | "Doe^John" |
| (0x0010, 0x0020) | Patient ID | "MRN123456" |
| (0x0008, 0x0020) | Study Date | "20240315" |
| (0x0008, 0x0030) | Study Time | "143000" |
| (0x0008, 0x0060) | Modality | "CT" (CT scan), "MR" (MRI), "US" (ultrasound) |
| (0x0028, 0x0002) | Samples Per Pixel | 1 (grayscale), 3 (color) |
| (0x0028, 0x0010) | Rows | 512 |
| (0x0028, 0x0011) | Columns | 512 |
| (0x7FE0, 0x0010) | Pixel Data | [bytes of image] |

### Series & Studies

DICOM organizes images hierarchically:

```
Study (one imaging encounter)
  Series 1 (chest X-ray, frontal view)
    Image 1
    Image 2
  Series 2 (chest X-ray, lateral view)
    Image 3
```

A patient may have multiple studies over time; each study has multiple series (different orientations, protocols); each series has multiple images (slices in CT/MRI).

### PACS (Picture Archiving and Communication Systems)

PACS server stores DICOM images, indexes by patient/study/series, enables retrieval for viewing.

**DICOM Network Protocol (DICOM over TCP/IP):**

```
Modality (CT scanner) -> PACS: Send completed study
Viewer (radiologist workstation) <- PACS: Retrieve study for reading
```

Uses DIMSE (DICOM Message Service Element) commands: Store, Retrieve, Query/Retrieve.

### DICOM Query/Retrieve (C-FIND, C-GET)

```
Query: Find all studies for patient MRN123456
  C-FIND request: (Patient ID = "MRN123456")
  PACS response: [Study1, Study2, Study3]

Retrieve: Get Study1 images
  C-GET request: Study Instance UID
  PACS response: [Stream of MR images]
```

## ICD-10 & SNOMED CT Coding

### ICD-10 (International Classification of Diseases, 10th Edition)

Used for **billing, epidemiology, mortality tracking**.

```
E11.9       Type 2 diabetes mellitus without complications
I10         Essential hypertension
J44.9       Chronic obstructive pulmonary disease, unspecified
Z79.4       Long-term (current) use of insulin
```

Structure:
- **1st char:** Letter (A-Z) indicating chapter (diseases, symptoms, etc.)
- **2-3rd chars:** Numeric, indicating category
- **4th char:** Decimal, indicating subcategory
- **5-7th chars:** Additional specificity (7 chars total max)

**Why it matters:** Codes map to reimbursement rates; wrong code = wrong payment. Billing errors often stem from miscoding.

### SNOMED CT (Systematized Nomenclature of Medicine Clinical Terms)

Used for **clinical documentation, semantic interoperability**.

Much more comprehensive than ICD-10; designed for EHRs.

```
73211009    |Diabetes mellitus (disorder)|
11891008    |Hypertension (disorder)|
65363002    |Ostearthritis, unspecified (disorder)|
```

**Attributes** link concepts semantically:

```
"Type 2 diabetes mellitus" 
  -- is-a --> "Diabetes mellitus"
  -- finding-site --> "Structure of pancreas"
  -- associated-morphology --> "Impaired glucose metabolism"
```

Enables inference: If a patient has "Type 2 diabetes," we can infer they have "Diabetes mellitus" via hierarchy.

### Mapping Between Systems

EHRs often map ICD-10 (for billing) to SNOMED CT (for clinical reasoning):

```
ICD-10: E11.9 (Type 2 diabetes)
SNOMED CT: 44054006 (Type 2 diabetes mellitus)
```

Maintains both for dual-use (billing + clinical).

## HIPAA & Healthcare Privacy/Security

### HIPAA (Health Insurance Portability & Accountability Act)

US federal law governing **Protected Health Information (PHI):**
- Patient demographics (name, address, date of birth)
- Medical record numbers
- Health conditions, medications, lab results
- Insurance information
- Any information identifiable to a patient

### HIPAA Rules

**Privacy Rule:** Patients have right to access their records. Covered entities can use/disclose PHI only for treatment, payment, operations, or with consent.

**Security Rule:** Implement safeguards (administrative, physical, technical) to protect ePHI (electronic PHI).

**Breach Notification Rule:** If PHI compromised, notify affected individuals within 60 days.

### PHI De-Identification

To share data for research/analysis without HIPAA compliance, remove identifying information:

**Safe Harbor Method (remove 18 identifiers):**
- Name
- Medical record number
- Health plan beneficiary number
- Account numbers
- Certificate/license numbers
- Vehicle identifiers & serial numbers
- Device identifiers & serial numbers
- Web URLs
- IP addresses
- Email addresses, phone/fax numbers
- Dates (except year) for persons >89 years
- Geographic subdivisions smaller than state
- Biometric identifiers
- Full-face photographic images

```
Before: "John Doe, MRN 123456, DOB 1980-01-15, diagnosed with diabetes on 2024-03-15"
After: "Patient X, diagnosed with diabetes"
```

**Expert Determination Method:** Statistician/HIPAA expert certifies data cannot re-identify patient with reasonable effort.

De-identified data is not subject to HIPAA.

### Security Technical Safeguards

- **Access controls:** Minimum necessary principle; role-based access
- **Encryption:** ePHI encrypted at rest (AES-256) and in transit (TLS 1.2+)
- **Audit logging:** All access/modifications logged with timestamp, user, action
- **Integrity:** Checksums/hashing detect corruption
- **Transmission security:** Secure channels (VPN, TLS) for sending PHI

HIPAA compliance typically requires SOC 2 Type II audit (covers security practices).

## EMR Integration Patterns

### Direct Protocol (Secure Email)

Exchange clinical documents (CCDs, PDFs) via Direct Protocol—encrypted email-like system for providers.

```
Provider A (EMR 1) -> Direct address (secure transport) -> Provider B (EMR 2)
```

Direct addresses look like email (user@domain.directsecure.com) but use stronger encryption than standard email.

### FHIR APIs for Integration

Modern integration: EMR A exposes FHIR APIs; EMR B queries or subscribes:

```
EMR B needs patient history from EMR A:
GET https://emra.com/fhir/Patient/12345/Condition
GET https://emra.com/fhir/Patient/12345/MedicationRequest
```

Enables real-time data sharing vs. nightly batch exports.

### CCD/CDA (Continuity of Care Document / Clinical Document Architecture)

XML structure for clinical summaries. Can embedded in FHIR:

```
FHIR DocumentReference -> Document (PDF)
CDA represents: Patient demographics, Problem list, Medications, Allergies, Procedures, Results Lab
```

Standard format enables portability across EMRs.

### Interoperability Challenges

- **Terminology mismatch:** One system uses ICD-10, another SNOMED CT; mapping imperfect
- **Data quality:** Unstructured notes (narrative text) don't map to structured fields
- **Scope:** FHIR covers core data but lacks specialty-specific extensions (e.g., oncology treatments)
- **Implementation burden:** Compliance often requires significant engineering

## Practical API Design for Healthcare

### Authentication & Authorization

Use OAuth 2.0 with SMART on FHIR (Substitutable Medical Applications and Reusable Technologies):

```
Provider logs into EHR
EHR redirects to authorization server
Authorization server issues access token with scopes:
  "patient/*.read" (read patient data)
  "Observation.read" (read observations)
  "MedicationRequest.write" (write prescriptions)
Provider's app can now call FHIR API with token
```

Token includes claims identifying provider & patient context.

### Audit Logging

Every PHI access must be logged:

```
{
  "timestamp": "2024-03-15T14:30:00Z",
  "user_id": "provider_123",
  "action": "READ",
  "resource_type": "Patient",
  "resource_id": "pat_12345",
  "outcome": "SUCCESS",
  "ip_address": "192.168.1.1"
}
```

Logs retained ≥6 years (varies by jurisdiction).

### Pagination & Performance

Health systems have millions of patients/observations. FHIR APIs must paginate:

```
GET /Patient?_count=100&_offset=0
Response:
{
  "resourceType": "Bundle",
  "entry": [
    {"resource": {"resourceType": "Patient", ...}},
    ...
  ],
  "link": [
    {"relation": "next", "url": "/Patient?_count=100&_offset=100"}
  ]
}
```

### Search Filters

```
GET /Observation?code=2085-9&date=ge2024-01-01&date=lt2024-03-15
// Cholesterol observations in Jan-Mar 2024

GET /MedicationRequest?status=active&patient=12345
// Active prescriptions for patient 12345
```

## See Also

- [API Design Principles](knowledge/api-design.md) — REST design for health APIs
- [Security — Compliance Frameworks: SOC 2, ISO 27001, PCI DSS, HIPAA, FedRAMP](knowledge/security-compliance-frameworks.md) — HIPAA in depth
- [Data — Structured Data Formats: JSON, XML, Protobuf](knowledge/data-structured-formats.md) — FHIR JSON encoding
- [Distributed Systems — Event Sourcing for Audit Trails](knowledge/architecture-event-sourcing.md) — audit logging for healthcare
- [Authentication & Authorization: OAuth 2.0, SAML, OIDC](knowledge/api-authentication.md) — SMART on FHIR OAuth pattern