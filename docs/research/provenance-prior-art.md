# Provenance and Manifest Prior Art for uncompose.project.json

## TL;DR

- W3C PROV gives the right mental model (Entity, Activity, wasDerivedFrom / used / wasGeneratedBy) but its full spec, RDF lineage, and JSON serializations are far heavier than a local manifest needs. Borrow the triple, not the framework.
- RO-Crate shows how to make a self-describing JSON file in a directory (`ro-crate-metadata.json`, version-in-context-URL, relative-path `@id`s) but also shows the friction JSON-LD adds. Borrow the conventions, skip JSON-LD.
- BWF bext is the cautionary tale for embedded metadata: any metadata edit rewrites the file, versioning is done by nibbling reserved bytes, and identifiers (OriginatorReference, UMID) live inside the thing they identify. External manifest is the right call.
- DAW session formats (AAF, OMF, DAWproject) model edits, automation, and device state. That is a different product. AAF's derivation chain via MobID references is the one idea worth noting.
- git-annex, DVC, and git LFS all converge on the same pattern: content hash is identity, path is location, pointer/manifest files are tiny text committed to git, and content lives in a hash-sharded cache. This is the strongest precedent for uncompose's verified/modified/missing model.
- C2PA's ingredient assertions and hashed references are worth borrowing conceptually; its embedded, signed, CBOR/JUMBF machinery exists for public authenticity claims and is out of scope.
- For schema evolution: JSON Schema `$id` with version in the URL, SPDX-style `SPDX-M.N` semantics for compatibility, git-LFS-style exact-string version comparison.

---

## 1. W3C PROV

Primary sources: [PROV-DM (W3C Recommendation)](https://www.w3.org/TR/prov-dm/), [PROV-JSON (W3C Member Submission)](https://www.w3.org/submissions/prov-json/), [PROV-O ontology](https://www.w3.org/TR/prov-o/).

### Core model

[PROV-DM](https://www.w3.org/TR/prov-dm/) defines three core types:

- **Entity**: "a physical, digital, conceptual, or other kind of thing with some fixed aspects."
- **Activity**: something occurring over time that "acts upon or with entities; it may include consuming, processing, transforming, modifying, relocating, using, or generating entities."
- **Agent**: "something that bears some form of responsibility for an activity taking place."

The three relations that matter for a derivation record:

- `wasGeneratedBy(entity, activity)`: the completion of production of a new entity by an activity.
- `used(activity, entity)`: the beginning of utilizing an entity by an activity.
- `wasDerivedFrom(derivedEntity, sourceEntity)`: "a transformation of an entity into another."

An uncompose derivation maps cleanly: input assets are `used`, output assets are `wasGeneratedBy`, and each output `wasDerivedFrom` its inputs. The tool invocation is the Activity. The user or tool is the Agent (optional).

### Identifiers

PROV identifiers are qualified names: a namespace prefix plus a local part, resolvable to IRIs (`entity(tr:WD-prov-dm-20111215, ...)`). Everything is designed for global, cross-system linking, which is why namespaces are mandatory. A single-project local manifest does not need IRIs; project-scoped string IDs suffice.

### Why full PROV is heavyweight

PROV-DM organizes concepts into [six components](https://www.w3.org/TR/prov-dm/): entities/activities, derivations (with Revision, Quotation, PrimarySource subtypes), agents/responsibility, bundles (provenance of provenance), alternate/specialized entities, and collections. Most of this exists to support open-world, multi-party, semantic-web provenance. A local manifest needs roughly 10 percent of it.

[PROV-JSON](https://www.w3.org/submissions/prov-json/) is a 2013 Member Submission, not a Recommendation ("Publication of this document by W3C indicates no endorsement of its content"). Its friction points are instructive:

- Typed literals need `{"$": value, "type": ...}` wrappers.
- Relations must have identifiers, forcing generated blank nodes like `_:A1`.
- "PROV-JSON makes no provision to ensure that records referred by equivalent identifiers will be merged."

### Takeaway

Use PROV vocabulary informally (inputs, outputs, activity) so the model is recognizable, but do not adopt PROV serialization, namespaced qualified names, or the extended component set.

---

## 2. RO-Crate

Primary source: [RO-Crate 1.2 specification](https://www.researchobject.org/ro-crate/specification/1.2/structure.html), [JSON-LD appendix](https://www.researchobject.org/ro-crate/specification/1.2/appendix/jsonld.html).

### Structure

An RO-Crate is a directory with a metadata file that "MUST be present in the RO-Crate Root and MUST be named `ro-crate-metadata.json`". The document is flattened, compacted JSON-LD:

```json
{ "@context": "https://w3id.org/ro/crate/1.2/context",
  "@graph": [ ] }
```

The `@graph` must contain a metadata descriptor, a root data entity, and zero or more data and contextual entities. The metadata descriptor declares `conformsTo` pointing at the versioned spec URI, so the spec version lives in a URL inside the document. The context URL is also versioned (`.../1.2/context`).

### File references

Data entities inside the crate use relative-path `@id`s (`./data/file.txt`); external resources use absolute URIs. The spec explicitly supports referencing content that is "large, require authentication or otherwise inconvenient to transfer with the RO-Crate." It also states the metadata is not an exhaustive inventory, just "sufficient metadata to understand and use the content." Both points match uncompose's reference-not-embed goal.

### Extension mechanism

RO-Crate reuses schema.org types and says implementers "MAY use terms from other vocabularies" when schema.org is insufficient. Custom terms are added by extending the context with the `@context: []` array form, either with project-hosted URIs or the shared `https://w3id.org/ro/terms/` namespace ([JSON-LD appendix](https://www.researchobject.org/ro-crate/specification/1.2/appendix/jsonld.html)).

### JSON-LD friction

The spec itself concedes the tension: "It is not necessary to use JSON-LD tooling to generate or parse the RO-Crate Metadata Document," and the context is "deliberately flat, listing every property and type" so tools can treat it as plain JSON. Even so, the appendix documents real traps: relative URI escaping, mandatory flattening of nested entities into the `@graph`, unpacking single-element arrays for compacted-form compliance, and contexts that need embedding "by value" to stay archivable offline. RO-Crate spends significant spec text making JSON-LD tolerable for people who just want JSON.

### Takeaway

Borrow: fixed metadata filename convention, version-in-URL (`$schema` analog of `conformsTo`), relative paths for local files, "describe what you need, not everything" scope, namespace-prefixed extension terms. Avoid: `@context`, `@graph`, and JSON-LD processing entirely. The uncompose non-goal "no RDF requirement" is exactly the lesson RO-Crate half-learned.

---

## 3. BWF / EBU embedded metadata

Primary sources: [EBU Tech 3285 v2 (BWF spec, PDF)](https://tech.ebu.ch/docs/tech/tech3285.pdf), [EBU Tech 3293 (EBUCore)](https://tech.ebu.ch/docs/tech/tech3293.pdf), [EBU Tech 3352 (identifiers in BWF)](https://tech.ebu.ch/docs/tech/tech3352.pdf).

### The bext chunk

BWF is RIFF WAVE plus a mandatory `bext` (Broadcast Audio Extension) chunk. Fields (Tech 3285 v2, section 2.3):

- `Description[256]`, `Originator[32]`: free ASCII.
- `OriginatorReference[32]`: "an unambiguous reference allocated by the originating organisation" (format standardized in EBU R99).
- `OriginationDate[10]` / `OriginationTime[8]`: creation timestamp.
- `TimeReference` (64-bit): first sample count since midnight.
- `Version` (WORD): version of the bext chunk itself.
- `UMID[64]`: SMPTE 330M Unique Material Identifier.
- Loudness fields (v2): integrated loudness, range, true peak, momentary, short-term, each a 16-bit int at 100x scale.
- `Reserved[180]`, then `CodingHistory[]`: CR/LF-separated strings, one appended per coding process applied to the audio (format in EBU R98). This is an append-only, in-file provenance log.

### Versioning by reserved bytes

Tech 3285 section 1.1 is a compact case study in binary-format evolution: Version 0 (1997) had 254 reserved bytes; Version 1 (2001) carved 64 of them into the UMID; Version 2 (2011) carved 10 more into loudness. Old software reads zeros where new fields live; the spec warns "users of such devices will lose metadata unless special precautions are taken." Forward and backward compatibility depend on readers checking the Version field, which the spec admits many do not.

### What embedded metadata does well and poorly

Well: metadata travels with the file through any copy or transfer, no sidecar to lose, and CodingHistory survives interchange between broadcasters.

Poorly:

- Any metadata edit rewrites the file, so a content hash over the whole file no longer identifies "the same audio." Fixing a typo in Description changes the SHA-256.
- Fixed-width ASCII fields and 100x-scaled integers are hostile to inspection and extension.
- Nothing verifies that CodingHistory matches what actually happened; it drifts because tools forget to append or copy stale chunks.
- Identifiers (OriginatorReference, UMID) are stored inside the thing they identify, so a tool that strips or rewrites chunks silently destroys identity.

### EBUCore

[Tech 3293](https://tech.ebu.ch/docs/tech/tech3293.pdf) is EBU's "Dublin Core for media": an XML (and later RDF ontology) descriptive metadata set with a generic `identifier` element of `identifierType`, aimed at broadcast asset management and FIMS. Its trajectory (v1.4 onward chasing Semantic Web alignment) mirrors PROV/RO-Crate: descriptive standards accrete RDF. Not directly applicable.

### Takeaway

Uncompose is right to keep the manifest external and to hash the file as-is. One consequence to document: if a user's tool rewrites bext or other chunks, the hash legitimately reports `modified`, because the bytes did change. That is a feature (the state model is honest), not a bug, but the docs should explain it since audio tools touch metadata chunks routinely.

---

## 4. DAW interchange: AAF, OMF, DAWproject

Primary sources: [AAF Object Specification v1.1 (AMWA)](https://static.amwa.tv/ms-01-aaf-object-spec.pdf), [AAF Edit Protocol (AMWA)](https://static.amwa.tv/as-01-aaf-edit-protocol-spec.pdf), [Library of Congress AAF format description](https://www.loc.gov/preservation/digital/formats/fdd/fdd000004.shtml), [bitwig/dawproject on GitHub](https://github.com/bitwig/dawproject).

### What session formats model

AAF (successor to the legacy, poorly documented OMF/OMFI) models compositions: Mobs (metadata objects) identified by globally unique MobIDs generated as SMPTE UMIDs, MobSlots, and SourceClip objects that weakly reference other Mobs by MobID plus a SlotID and time offset. The [AAF object spec](https://static.amwa.tv/ms-01-aaf-object-spec.pdf) states "the derivation chain is specified by referencing one material object from another using SourceClip objects" and that an exporter "should include and reference the Mobs for the entire derivation chain to the extent that it is aware of it." CompositionMobs carry the edit decisions.

[DAWproject](https://github.com/bitwig/dawproject) is the modern open equivalent: a `.dawproject` ZIP containing `project.xml` and `metadata.xml` (XML Schema defined, MIT licensed, v1.0, supported by Bitwig, Studio One, Steinberg hosts and others). It models tracks, clips, fades, warping, note expressions, automation of tempo/volume/pan/sends/plugin parameters, and full plugin state. Audio can be embedded in the ZIP or referenced externally; the exporting DAW chooses the internal directory layout, and importers must handle both.

### Why uncompose is explicitly not this

Session formats answer "how do I reconstruct the edit?" They model timelines, automation curves, and device state, and they pay for it with large object models (AAF's structured-storage container and hundreds of classes) or a ZIP that is no longer diffable or inspectable in git. Uncompose answers "which file came from which files via which tool run?" and needs none of the timeline machinery. DAWproject's embed-or-reference ambivalence is also a warning: two valid projects for the same material, and referenced-audio projects silently break on move because references are paths with no content hash.

### Worth noting from AAF

The derivation chain by stable ID reference (CompositionMob -> MasterMob -> file source Mob) is structurally the same idea as derivation records referencing asset IDs. AAF proves the pattern works at industrial scale; it also proves you do not need to store the referenced content to record the chain.

---

## 5. Content addressing: git-annex, DVC, git LFS

Primary sources: [git-annex key format](https://git-annex.branchable.com/internals/key_format/), [git-annex how it works](https://git-annex.branchable.com/how_it_works/), [DVC .dvc files](https://doc.dvc.org/user-guide/project-structure/dvc-files), [DVC internal files](https://doc.dvc.org/user-guide/project-structure/internal-files), [git LFS pointer spec](https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md).

### git-annex

Keys have the form `BACKEND[-sSIZE][-mMTIME]--NAME`, for example `SHA256E-s31390--f50d...cc0.mp3`. Backend, size, and hash are all in the key; the `E` backends append the file extension to the hash so the key preserves the media type, which matters for tools that sniff extensions. The working tree contains git-committed symlinks pointing into `.git/annex/objects`; content changes produce a new key and a retargeted symlink. Location tracking (which clone has which key) lives in a dedicated `git-annex` branch. Identity is fully separated from location: the key never changes when files move, and any path can point at any key.

### DVC

A `.dvc` file is a small YAML sidecar committed to git, listing `outs` entries with `path`, a hash (`md5` locally, `etag` for cloud remotes), `size`, and `nfiles` for directories. Content lives in a hash-sharded cache: `.dvc/cache/files/md5/ec/1d2935f8...` ("the first two characters are used to name the directory inside the cache"). Directories get a `.dir` JSON file mapping file hashes to relative paths. `dvc checkout` restores workspace paths from the cache via reflink/hardlink/symlink/copy. `dvc.lock` plays the same role for pipeline stages: recorded hashes of deps and outs pin exactly what a stage consumed and produced. The design point relevant here: the schema "separates metadata (hashes, sizes, paths) from the actual cached content, enabling DVC to track versions independently of file locations."

### git LFS

The [pointer file spec](https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md) is the minimal case: a UTF-8 text file with exactly `version` (a URL, `https://git-lfs.github.com/spec/v1`), `oid sha256:<lowercase hex>`, and `size`, keys alphabetical except version first, max 1024 bytes. Versioning is "simple string comparison on the version, without any URL parsing or normalization." Content is stored at `.git/lfs/objects/{first2}/{next2}/{full-oid}`.

### Repair and relink on move

All three treat a moved file as a location problem, not an identity problem. git-annex fixes the symlink; DVC re-links from cache on checkout; LFS never cares where the working-tree path is because the pointer travels with it in git. Uncompose's equivalent: when a path is missing, scan candidate files, hash them, and relink any whose SHA-256 matches a known asset. Storing `size` next to the hash (as all three do) makes the scan cheap: size mismatch rules a file out without hashing.

### Takeaway

This family is the closest prior art to uncompose's integrity model. Hash plus size as identity, path as a mutable hint, tiny text records in git, and mechanical relink on move are all proven. Uncompose differs only in not maintaining a content cache; the audio files themselves are the store, so `missing` is a real state rather than a checkout trigger.

---

## 6. C2PA

Primary source: [C2PA Specification 2.1](https://spec.c2pa.org/specifications/specifications/2.1/specs/C2PA_Specification.html).

### Model

A C2PA Manifest Store is "a collection of C2PA Manifests that can either be embedded into an asset or be external to its asset," serialized in JUMBF boxes. Each manifest contains assertions (labeled declarations, e.g. `c2pa.actions.v2`, duplicates disambiguated as `c2pa.metadata__1`) and a claim (`c2pa.claim.v2`, deterministic CBOR) that gathers assertion references and is signed with COSE over X.509 certificates, ideally with RFC 3161 timestamps.

### Hard bindings

Content integrity comes from hard binding assertions: `c2pa.hash.data` (byte ranges), `c2pa.hash.boxes`, or `c2pa.hash.bmff.v3` depending on format, with the rule that "a single manifest shall not contain more than one assertion defining a hard binding." Byte-range hashing exists precisely because of the BWF problem above: it lets the hash exclude the embedded manifest itself and other mutable regions.

### Ingredients

Ingredient assertions describe source assets used to build a derived or composed asset, with standardized relationship types (parent vs component). This is C2PA's derivation record: each ingredient carries its own hashed reference, so the chain is verifiable link by link. External manifests are referenced via `hashed-ext-uri-map` structures (URI plus hash of what the URI should contain), and internal references use `self#jumbf` URIs.

### Identifiers and versioning

Every manifest gets a URN: `urn:c2pa:<UUIDv4>:<generator>:<version_reason>`. Assertion schemas version by label increment (`c2pa.actions` -> `c2pa.actions.v2`); backward-compatible field additions do not bump the label, breaking changes get a new label.

### Why it is not the model for uncompose

C2PA's machinery (CBOR, JUMBF embedding, X.509 PKI, timestamping) exists to make claims verifiable by strangers across distribution channels. Uncompose is local-first with a trusted operator; signing and embedding buy nothing and cost inspectability.

### Worth borrowing

- The hashed-reference pattern: any pointer to an external record (uncompose's `job.json`) should carry the hash of that record, so the link is verifiable like a C2PA `hashed-ext-uri-map` entry.
- The ingredient relationship vocabulary: distinguishing "parent" (the thing this was derived from) from "component" (an input mixed in) is a useful refinement if derivations ever need typed inputs.
- Label-based assertion versioning as an alternative to schema-wide version bumps for individual record types.

---

## 7. Schema versioning conventions

Primary sources: [JSON Schema: structuring](https://json-schema.org/understanding-json-schema/structuring), [SPDX 2.3 document creation information](https://spdx.github.io/spdx-spec/v2.3/document-creation-information/).

- JSON Schema: `$schema` declares the dialect the schema is written in; `$id` is the schema's own identity and base URI, and the guidance is to "always use an absolute URI when declaring a base URI with `$id`" and to put version information in the URL so evolution does not break existing references.
- SPDX: documents carry `spdxVersion` as `SPDX-M.N` where "the major field shall be incremented when incompatible changes between versions are made" and minor for backward-compatible ones. Element IDs are document-scoped strings (`SPDXRef-DOCUMENT`, `SPDXRef-<idstring>`); the document namespace is a unique absolute URI that "does not have to be accessible."
- git LFS (above): version as an exact-match URL string, compared with plain string equality. The simplest scheme that works.
- BWF (above): the counterexample. Versioning by consuming reserved bytes plus a version field readers ignore leads to silent metadata loss.

Pattern across all of the healthy examples: the version is inside the document, machine-comparable, and semantically split into breaking vs additive.

---

## What uncompose.project.json should borrow / avoid

### Identifiers

- Project ID: a ULID (or UUIDv4 per C2PA practice) minted once at project creation. Globally unique without coordination, stable forever, sortable if ULID.
- Asset IDs: human-readable, project-scoped strings (`vocals-dry`, `mix-v3`) chosen by the user or tool. PROV and SPDX both show document-scoped IDs are fine when the document itself has a global ID. Do not use paths as IDs (breaks on move) and do not use raw hashes as IDs (unreadable, and re-encoding a file would orphan its history).
- Rule: IDs are immutable once referenced by a derivation or evaluation, same as AAF MobIDs.

### Identity vs location

- SHA-256 of the file bytes is identity; the path is a location hint, exactly the git-annex/DVC/LFS split. Store `size` alongside the hash (all three do) so integrity checks and relink scans can short-circuit without hashing.
- States fall out mechanically: path exists and hash matches = `verified`; path exists and hash differs = `modified`; path absent = `missing`. On `missing`, offer a relink scan: hash candidate files (size-filtered first) and reattach any match, the manifest equivalent of git-annex symlink repair.
- Document the BWF lesson explicitly: metadata-chunk edits by audio tools legitimately flip an asset to `modified`. Consider recording an optional secondary audio-data-only hash later if this becomes painful, but do not complicate v1; C2PA needed byte-range hashing only because it embeds.

### Derivations

- Use the minimal PROV triple per derivation: inputs (`used`), outputs (`wasGeneratedBy`), and the activity (tool name, version, parameters, timestamp). `wasDerivedFrom` is then derivable and need not be stored. Optional agent field for who/what ran it.
- Reference `job.json` rather than absorbing it, and store the SHA-256 of the referenced `job.json` next to the path (C2PA's hashed-ext-uri pattern). The manifest stays small and diffable; the execution record stays verifiable.
- Do not adopt PROV bundles, collections, specialization, or qualified names. AAF shows a plain chain of ID references carries industrial workloads.

### Schema versioning and extensions

- Put the version in a `$schema`-style absolute URL (`https://uncompose.org/schema/project/v1.json`), compared as an exact string like git LFS. The URL need not be fetchable to validate (SPDX namespace precedent), but publishing a real JSON Schema there is cheap and worth doing.
- Major version in the URL for breaking changes; additive fields do not bump it (SPDX minor semantics). Never repurpose existing fields (the BWF reserved-bytes trap).
- Namespaced extension fields (`"x-mytool:whatever"` or a top-level `extensions` object keyed by namespace) with the rule that unknown namespaces are preserved on rewrite, like BWF's "compliant applications shall pass these chunks" but with the pass-through requirement made explicit and testable. Borrow RO-Crate's spirit (extend without forking) without its context machinery.

### Evaluations and file layout

- Keep evaluations inline in the manifest while they are small structured records (a comparison of asset IDs plus verdict and notes); they are the part humans most want to read in a diff. Move to referenced files with hashes only if they grow blobs (long transcripts, exported metrics), applying the same hashed-reference rule as `job.json`. RO-Crate's "sufficient metadata, not an inventory" is the scope test.
- Fixed filename `uncompose.project.json` at the project root, RO-Crate style, so tools can discover it without configuration.
- Plain JSON, stable key ordering on write, one entity per line where practical, so git diffs stay reviewable. No JSON-LD, no RDF, no embedding audio. Every prior-art family that added semantic-web machinery (PROV, RO-Crate, EBUCore 1.4+) spent spec effort apologizing for it afterward.
