# Uncompose — Project Kickoff Brief

## Project purpose

Uncompose is an open-source music source-separation project built to give musicians direct ownership and control over the process of separating finished recordings into vocals, drums, bass, and other instrumental parts.

The project begins with a concrete personal need: replacing a Moises subscription with a capable local alternative. It is also intended to become a meaningful, long-lived open-source project—one that demonstrates thoughtful product development, modern software engineering, effective use of AI-assisted workflows, and responsible project stewardship.

Uncompose should be useful as a product first and impressive as a portfolio project because of how well it is conceived, built, documented, and maintained.

## High-level product definition

Uncompose is a local-first, open-source music-separation platform for musicians, developers, and technically curious users who want to extract and work with the constituent parts of recorded music.

At its simplest, a user should be able to provide an audio file, select a separation model or preset, run the process, audition the resulting stems, and export them.

Over time, Uncompose may grow into a broader workbench for understanding and transforming recorded music, including:

* Multiple interchangeable separation models
* CPU and GPU processing
* Model and quality comparisons
* Stem playback, muting, soloing, and mixing
* Batch processing
* Export presets
* Command-line, API, and graphical interfaces
* Practice-oriented workflows
* Integration with tools such as Etudely, DAWs, and other open music software

The first release should remain focused: reliably separate a recording and make the results easy to use.

## Problem and opportunity

Commercial services make music separation convenient, but they commonly introduce subscriptions, upload requirements, processing limits, proprietary workflows, and limited control over the underlying models.

Existing open-source separation models are powerful, but using them often requires technical knowledge and fragmented tooling. There is room for a polished project that makes these capabilities approachable without hiding or restricting them.

Uncompose should bridge that gap:

> Moises is a service musicians use. Uncompose is a tool they own.

## Intended users

### Primary user

A musician who wants to isolate or reduce parts of a recording for practice, transcription, remixing, production, study, or experimentation.

The initial primary user is the project’s creator. Early product decisions should solve real musical workflows rather than imagined market requirements.

### Secondary users

* Producers and audio engineers
* Music educators and students
* Developers building music-related applications
* Researchers experimenting with separation models
* Open-source contributors interested in audio, machine learning, interfaces, infrastructure, or documentation

## Product positioning

Uncompose is not merely a wrapper around a single source-separation model.

It should become a coherent product and platform that handles the complete workflow around separation:

1. Accept and validate audio
2. Select and configure a model
3. Execute processing
4. Report progress and failures clearly
5. Preserve reproducible job information
6. Let users audition and inspect results
7. Export stems in useful formats
8. Make the workflow accessible through stable interfaces

Uncompose should differentiate itself through local ownership, model flexibility, usability, transparency, and engineering quality.

It should coexist with adjacent projects rather than positioning itself as an unnecessary replacement for all of them. For example, OpenStems currently focuses more heavily on live stem manipulation and integration with music players, OBS, and DAWs. Uncompose’s initial center of gravity is offline or local processing, model orchestration, resulting stem management, and musician-centered workflows.

## Open-source mission

Uncompose should be genuinely open source, not merely source-available marketing for a hosted product.

Its open-source mission is to:

* Give users control over their audio and processing environment
* Make modern source-separation technology accessible
* Support multiple models rather than creating artificial lock-in
* Document architectural decisions and tradeoffs openly
* Welcome contributions across engineering, audio, design, documentation, and testing
* Build a healthy project that can outlive its original implementation choices
* Treat contributors and users as participants rather than leads in a sales funnel

The project may eventually support hosted or commercial offerings, but the core local workflow should remain useful on its own.

## Stewardship principles

This is intended to be the first open-source project that Dominic Hanzely actively stewards rather than simply publishes.

Good stewardship should include:

* A clear project vision
* A documented contribution process
* Respectful and timely issue management
* Transparent roadmap and decision-making
* Well-scoped contribution opportunities
* Useful documentation for both users and developers
* Stable releases and meaningful versioning
* Explicit licensing and governance
* Honest communication about limitations
* Responsible handling of model licenses and copyrighted audio

The quality of the community experience should be treated as part of the product.

## Product principles

### Local first

Users should be able to process their own audio without uploading it to an external service.

### Musician centered

The project should be designed around musical tasks and outcomes, not around exposing machine-learning terminology for its own sake.

### Model agnostic

Separation models should be treated as interchangeable backends behind a consistent workflow wherever practical.

### Progressive complexity

The simplest useful workflow should remain simple, while advanced users should be able to inspect and control deeper settings.

### Transparent and reproducible

Users should be able to determine which model, version, parameters, and environment produced a result.

### Composable

The project should expose capabilities through interfaces that other software can use, potentially including a CLI, library, API, and event or job system.

### Open without being unfinished

Open-source software can still feel coherent, intentional, documented, and polished.

## Brand identity

### Name

**Uncompose**

The name describes the conceptual inverse of musical composition: taking a unified recording and revealing its constituent performances.

It also has a subtle software meaning, which may resonate with developers, although the musical context should always be made clear.

### Working descriptor

**Open-source music source separation**

### Working tagline

**Take the mix apart.**

### Longer positioning line

**Separate recorded music into vocals, drums, bass, and instruments—locally, openly, and without a subscription.**

### Brand character

Uncompose should feel:

* Musical rather than clinical
* Technically serious without being intimidating
* Independent and open
* Precise, calm, and purposeful
* Slightly experimental, but not disposable or gimmicky

The visual identity can be developed later. Early branding should remain restrained and let the name, typography, product experience, and documentation establish credibility.

## Project identity and ownership

Working public structure:

* Product: **Uncompose**
* Website: **uncompose.cc**
* Repository: **github.com/thedahm/uncompose**
* Creator and maintainer: **Dominic Hanzely**
* Possible future affiliation: **An open-source project from 0x13.cc**

Keeping the repository under the personal `thedahm` account makes authorship and stewardship immediately visible. The independent domain allows the project to develop its own identity and remain portable if its organizational home changes later.

## Initial product scope

The first meaningful version should prove the complete core workflow:

* Install or launch Uncompose
* Select an audio file
* Choose a supported separation preset
* Run the separation locally
* Observe meaningful progress
* Receive clearly organized output stems
* Audition the results
* Export or locate the generated files
* Review basic information about the completed job

The first version should support one excellent path before supporting many mediocre paths.

A likely initial model integration may be Demucs or another established open-source separator, but model selection should be evaluated during technical planning rather than assumed by the brand brief.

## Potential initial interfaces

The project should eventually support more than one interface, but the order should be chosen deliberately.

Possible surfaces include:

* Command-line interface
* Local web application
* Python library
* HTTP API
* Desktop application
* Container image

The planning session should determine which interface best establishes the core architecture while still producing something musicians can use early.

## Explicit early non-goals

Unless technical discovery changes the conclusion, the initial project should not attempt to:

* Train a new separation model
* Match every Moises feature
* Become a complete DAW
* Provide a large hosted processing service
* Support every operating system and accelerator immediately
* Solve real-time separation in the first release
* Build account, billing, or subscription systems
* Add transcription, chord recognition, or practice features before the separation workflow is solid

These may become valid later directions, but they should not dilute the first milestone.

## Engineering and portfolio goals

Uncompose should showcase more than the final user interface. It should demonstrate the ability to design and steward a modern software system.

The project should provide evidence of:

* Clear domain modeling
* Thoughtful architectural boundaries
* Reliable job and process management
* Strong API and interface design
* Testing across model, orchestration, and user-facing layers
* Hardware-aware processing
* Useful observability and diagnostics
* Secure handling of user files
* Reproducible environments
* Automated builds, releases, and documentation
* Effective use of AI agents without surrendering engineering judgment
* Maintainer-quality issue triage and project communication

AI-assisted development should be visible through good outcomes, documented workflows, and disciplined review—not through unnecessary claims that the project is “AI-powered.”

## Measures of early success

The project’s first phase is successful when:

1. Dominic can use Uncompose instead of Moises for a meaningful portion of his own separation needs.
2. A technically competent user can install and run it from the documentation alone.
3. A musician can understand what it does without understanding machine learning.
4. A completed separation job is reproducible and diagnosable.
5. The architecture can support a second model without major redesign.
6. The repository presents a coherent, credible open-source project.
7. At least one outside contributor can understand how to make a useful contribution.
8. The project is strong enough to share publicly as representative professional work.

## Decisions for the kickoff planning session

The planning session should resolve or substantially narrow the following:

### User experience

* What is the first exact workflow Uncompose will support?
* Is the first usable interface a CLI, local web UI, or both?
* What inputs, stem presets, and output formats are required?
* What would make the first version personally useful enough to replace Moises?

### Model strategy

* Which model should be integrated first?
* What are its licensing, hardware, quality, and installation implications?
* What abstraction is needed to support additional models later?
* Should model installation be bundled, automatic, or separately managed?

### System architecture

* What are the core domain concepts?
* How should jobs, models, inputs, outputs, and artifacts be represented?
* Should processing run directly, through worker processes, or through a queue?
* How should progress, cancellation, retries, and failures work?
* Which boundaries should exist between core logic, model adapters, interfaces, and persistence?

### Distribution

* What operating system and hardware combination is supported first?
* Should the first release use Python packaging, containers, native installers, or another mechanism?
* How will large model files be acquired and versioned?
* What is the simplest reliable contributor setup?

### Repository foundation

* License
* Code of conduct
* Contribution guide
* Governance and maintainer expectations
* Issue and pull-request templates
* Release and versioning strategy
* Documentation structure
* Decision-record format
* Security and copyright guidance

### Initial milestones

* What constitutes the first technical spike?
* What constitutes the first personally usable build?
* What constitutes the first public release?
* What work should explicitly be deferred?

## Proposed kickoff outcome

The kickoff planning session should produce:

1. A one-paragraph product charter
2. A defined first user workflow
3. An initial model choice and technical spike
4. A lightweight architecture proposal
5. A repository and licensing decision
6. A first-release scope and non-goals
7. A milestone-based roadmap
8. A short list of architectural risks and open questions
9. A definition of done for the first public release

The goal is not to design the entire future of Uncompose. It is to establish enough direction to begin building confidently while preserving room for discovery.

## Working project statement

> Uncompose is a local-first, open-source music source-separation platform created and maintained by Dominic Hanzely. It helps musicians separate recorded music into vocals, drums, bass, instruments, and other parts while retaining control over their audio, models, and workflow.
