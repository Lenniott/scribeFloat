# Terms of Use — ScribeFloat

**Version 1.0 | Last updated: 2026-05-23**

ScribeFloat is **free, open source, and offered as-is**. You can use it for personal or commercial work without paying anything. These terms exist to keep the project honest: use it, share it, modify it — just don't strip the credit, claim it as your own work, or resell it as a closed product without attribution. The software is released under the [MIT License](LICENSE); the terms below restate and clarify it for end users.

By downloading, installing, or running ScribeFloat you accept these terms. If you don't, don't use the app.

---

## 1. What you can do

ScribeFloat is open source software released under the MIT License. That license already grants you broad rights — this section summarises them in plain English so you don't have to read the legal text first.

- Use ScribeFloat for any purpose — personal, commercial, internal business, research, education.
- Run it on as many machines as you want, for as many users as you want. There is no licence count.
- Read, modify, and rebuild the source code.
- Distribute copies of the original or your modified version.
- Bundle ScribeFloat (or parts of it) inside your own product, paid or free.

No account, no licence key, no activation server. Nothing phones home. The download you receive is the entire product.

---

## 2. What you can't do

The MIT License has one substantive requirement: the copyright notice and licence text must be included in any copy or substantial portion of the software. In practical terms that means:

- Don't strip the copyright notice from the source code, the `LICENSE` file, or any binary distribution that bundles ScribeFloat code.
- Don't claim that you wrote ScribeFloat, or that ScribeFloat is your original work. Forks and derivatives are welcome — but credit the upstream project and keep the licence intact.
- Don't resell ScribeFloat as a closed-source product without attribution. You can charge for distribution, support, or value-added features — but the underlying ScribeFloat code remains MIT-licensed and credited.
- Don't use the ScribeFloat name, logo, or branding to imply endorsement of a fork or derivative that isn't actually maintained by the ScribeFloat project.
- Don't use ScribeFloat to break the law, including recording laws (see section 4).

> **Short version:** fork it, ship it, build on it — just don't pretend you wrote it, and don't remove the credit.

---

## 3. No warranty, no liability

ScribeFloat is provided "as is", without warranty of any kind, express or implied. This includes — but is not limited to — warranties of merchantability, fitness for a particular purpose, and non-infringement.

The author and contributors are **not liable** for any claim, damages, or other liability arising from your use of the software. That covers, for example:

- Lost, corrupted, or incorrectly transcribed recordings.
- Missed or mis-heard audio (Whisper is a probabilistic model — it makes mistakes).
- Decisions, actions, or business outcomes based on a transcript ScribeFloat produced.
- Legal consequences of recording a call or meeting without the consent required by your jurisdiction (see section 4).
- Data loss, system damage, or service interruption caused by running the app.

If accuracy or legal compliance matters for your use case, verify transcripts manually and follow the rules that apply where you live and work. **You are responsible for how you use this tool.**

---

## 4. Recording laws — your responsibility

ScribeFloat can record audio from your microphone and (optionally) from system audio output. Recording other people speaking is regulated almost everywhere, and the rules vary widely by jurisdiction. Before you record a conversation that includes anyone other than yourself, check the law that applies to you and to every other participant.

### 4.1 Examples (not legal advice)

- **United States** — federal law and most states allow recording with the consent of at least one party to the conversation ("one-party consent"). Several states — including California, Florida, Illinois, Massachusetts, Pennsylvania, and Washington — require consent from all parties ("all-party consent" or "two-party consent"). The stricter rule usually applies when participants are in different states.
- **Canada** — recording a private conversation is generally legal if at least one party consents.
- **United Kingdom** — recording your own conversations for personal use is generally permitted; sharing, broadcasting, or using a recording for business purposes can engage UK GDPR and the Data Protection Act 2018.
- **European Union / EEA** — recordings that capture identifiable voices are personal data under GDPR. You typically need a lawful basis (often consent) and must inform participants. National laws vary.
- **Australia** — rules differ by state and territory; several require consent from all parties.
- **Japan, Singapore, India, and many other jurisdictions** — rules vary and may overlap with wiretapping, privacy, or workplace surveillance statutes.

### 4.2 Tell people you're recording

Even where the law only requires one-party consent, the courteous and professional default is to **tell everyone on the call that you're recording**, and to give them the chance to object. For meetings, calls, interviews, and any conversation that isn't obviously public, announce the recording up front and get a clear acknowledgement before you start capturing.

ScribeFloat does not display a notification to the people you are recording. The Scribe mode runs in the background by design. It is on you — the person running the app — to obtain the consent your jurisdiction requires and to honour any request to stop recording.

> **Bottom line:** the recording is yours, the legal exposure is yours, and the responsibility to inform other participants is yours. None of the above is legal advice — when in doubt, consult a lawyer in your jurisdiction.

---

## 5. Third-party components

ScribeFloat bundles or downloads open-source components owned by third parties. Their licences apply to those components.

- OpenAI Whisper (MIT) — the speech-recognition model.
- Silero VAD (MIT) — optional voice activity detection model.
- ggml / whisper.cpp (MIT) — local inference engine.
- Tauri, Svelte, and other Rust / JavaScript dependencies — each under their own open-source licence.

Model files are downloaded directly from Hugging Face on user request. Those downloads are subject to Hugging Face's terms and the model authors' licences.

---

## 6. Privacy

ScribeFloat does not collect telemetry, analytics, or any user data. Audio and transcripts stay on your device. See [PRIVACY.md](PRIVACY.md) for the full data-flow audit.

---

## 7. Changes to these terms

These terms may be updated as the project evolves. The current version, including the version number and "last updated" date, is always visible at the top of this document. Material changes will be reflected in the commit history on the project repository.

---

## 8. Contact

Questions, takedown requests, security reports, or licensing clarifications: open an issue on the [GitHub repository](https://github.com/Lenniott/scribeFloat).
