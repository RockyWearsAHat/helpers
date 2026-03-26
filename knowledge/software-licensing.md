# Software Licensing — Open Source Licenses, Copyleft, Compatibility & Compliance

## Overview

Software licenses govern how code can be used, modified, and shared. For open source developers, choosing a license is critical: it determines whether others can build on your work, whether they must share improvements, and whether they need legal permission.

For producers consuming open source, licenses impose obligations: if you include GPL code, must you open-source your whole product? If you modify Apache code, what notices must you include?

This note covers **open source licenses**, their terms, compatibility, and practical considerations. Proprietary/commercial licensing is outside scope.

## The Core Spectrum: Permissive vs. Copyleft

Open source licenses sit on a spectrum from **permissive** to **copyleft**.

**Permissive licenses** (MIT, Apache 2.0, BSD): You can do almost anything — use, modify, commercialize — with minimal obligations. The main condition: preserve the license and copyright notice. Permissive licenses **maximize downstream freedom** but don't guarantee improvements flow back to the original project.

**Copyleft licenses** (GPL, AGPL, LGPL): You can use and modify freely, but if you distribute your version, you must make the source available under the same license. This creates a **mirror obligation**: take code under GPL, distribute modifications, and you must release your derivative under GPL too.

**Middle ground (Weak copyleft)**: LGPL, MPL. More permissive than GPL but stronger than MIT. Conditions apply mainly to modifications or certain distribution methods.

Trade-offs:
- Permissive → easier for businesses (can incorporate without opening code) but may not ensure the project benefits
- Copyleft → ensures improvements flow back and communities stay open, but may be uncomfortable for proprietary workflows

## Permissive Licenses

### MIT License

**Terms**: Use, modify, sublicense, sell — with just two requirements: (1) retain the license and copyright notice, (2) provide as-is with no warranty.

**Length**: ~25 lines. Simplest open source license.

**Popular because**: Minimal friction. Businesses love it. Allows closing the code in derivatives. GitHub's default recommendation.

**Use when**: You want adoption and don't care if improvements are shared back.

### Apache License 2.0

**Terms**: Similar to MIT but adds an explicit patent grant. "If contributor X has a patent that applies to their code, they grant a license to practice that patent." This is crucial for companies: no patent litigation risk over contributions.

**Additional clause**: Requires notices of modifications, disclaims trademarks, and discloses changes (more prescriptive than MIT).

**Length**: ~200 lines (long but very legible).

**Popular because**: Strong patent protection (valuable at companies), clear modification procedures.

**Use when**: You're concerned about patent disputes or want corporate-style clarity.

### BSD Licenses

**2-Clause BSD**: Like MIT but requires a disclaimer on advertising (outdated). Rarely used now.

**3-Clause BSD**: Like 2-Clause but adds a clause prohibiting use of the author's name in derived projects without permission. Minor but pedantic.

**4-Clause BSD** (rarely used): Adds requirement to acknowledge contribution in advertising.

BSD is permissive like MIT but feels dated. MIT has largely superseded it.

## Copyleft Licenses

### GPL v2

**Terms**: Use and modify freely, but if you **distribute** (ship, deploy) your modified version, you must make the source available under GPL v2 to your recipients.

**Critical point**: "Distribution" triggers the obligation. If you modify GPL code but only use it internally (don't distribute), you don't have to release source.

**Consequence**: If Product X incorporates GPL v2 code and ships Product X, the vendor must provide Product X's full source under GPL v2.

**Popular for**: Linux, GCC, GDB, MySQL (early versions).

**Limitations**: No explicit patent grant (unlike Apache 2.0). Vague on some modern scenarios (SaaS, cloud APIs — is running code on a server "distribution"?).

### GPL v3

**Terms**: Same as GPL v2 but with important additions:

- **Explicit patent grant**: Contributors grant patents needed to use their code.
- **Tivoization clause**: "You can't distribute GPL v3 code in hardware that prevents modification." (If someone sells you a device with GPL v3 code, you must be able to modify the firmware. This blocked Broadcom WiFi drivers, NVIDIA firmware, etc.)
- **Compatibility with more licenses**: GPL v3 is compatible with Apache 2.0 (v2 was not).

**Adoption**: Linux kernel stayed on GPL v2 (avoiding tivoization; many believe it's too restrictive). Google prefers GPL v2 for the same reason. FSF recommends GPL v3 for new projects.

**Trade-off**: GPL v3 is stronger at ensuring freedom (hardware, SaaS) but less popular than v2.

### LGPL (Lesser GPL) v2.1 & v3

**Intent**: Copyleft for libraries, without forcing whole applications to be open source.

**Terms**: If you **link against** an LGPL library (static link), you must provide the object files so the recipient can re-link with a modified library (allowing library changes). If you **dynamically link** (standard case), no obligation if the .so/.dll is unchanged.

**Consequence**: Engineers can use LGPL libraries in proprietary products as long as they allow the recipient to update the library (e.g., link to a newer version). This is feasible for dynamically linked libraries but awkward for static linking.

**Popular for**: glibc, Qt (dual-licensed), many utilities.

**Use when**: You want libraries open source but don't want to force applications using them to open source.

### AGPL v3

**Terms**: Like GPL v3, plus: if you **run the software over a network** (SaaS, web service), you must make source available to network users.

**Consequence**: Run AGPL code as a web service, and you must release the source (or pay for a commercial license).

**Rationale**: GPL v3 has a "SaaS loophole" — you can run modified GPL code on a web server without distributing source to users. AGPL closes this. But this is **controversial**: many see AGPL as too aggressive and avoid it.

**Popular for**: Some infrastructure projects (Discourse, GitLab Community Edition).

**Use when**: You want to ensure SaaS derivatives stay open source (acknowledge the cost to adoption).

### MPL 2.0 (Mozilla Public License)

**Terms**: File-level copyleft. Files under MPL must remain under MPL if modified. You can combine with proprietary code (e.g., a browser with MPL core and proprietary plugins).

**Advantage**: More flexible than GPL (file-level, not project-level). You can add proprietary features without opening the entire product.

**Popular for**: Firefox, Thunderbird.

**Use when**: You want open source core with possible proprietary add-ons.

## License Compatibility & Mixing

Combining code under different licenses is a minefield. Questions:

- **Can I distribute GPL + Apache code together?** GPL v2 + Apache 2.0 = **conflicting**. GPL v2 doesn't understand Apache patents, and combining them creates ambiguity. Solution: license your code under GPL v2 + Apache 2.0 (allowing both). Technically impossible due to terms, but some projects do it anyway (legally murky).
- **Can I distribute MIT + Apache?** Yes, MIT is compatible with Apache. Include both notices.
- **Can I distribute GPL v2 + GPL v3?** **No**, they're incompatible. GPL v2 code can't automatically be upgraded to v3. (You can distribute both if you're the copyright holder and explicitly dual-license.)

**Compatibility matrix** (Can I combine these licenses in one project?):
- MIT + Apache 2.0 ✓ (both permissive)
- MIT + GPL v2 ✓ (MIT is more permissive, compatible with anything)
- Apache 2.0 + GPL v2 ✗ (patent grants conflict)
- GPL v2 + GPL v3 ✗ (incompatible terms)
- GPLv3 + Apache 2.0 ✓ (explicitly compatible in GPL v3)
- LGPL v3 + Apache 2.0 ✓
- LGPL + proprietary ✓ (if dynamically linked)

Tools like **FOSSA** and **Black Duck** scan code for license conflicts and flag compatibility issues.

## Practical Considerations

### Choosing a License for Your Project

**Open source for adoption?** → MIT (lowest friction) or Apache 2.0 (patent protection).

**Strategic importance to your future?** → GPL v3 (ensures improvements stay open) but accept lower adoption.

**Library that proprietary projects will use?** → MIT, Apache 2.0, or LGPL (don't force their whole product open).

**Infrastructure project (database, etc.)?** → MongoDB, Redis, and others have moved to **Commons Clause** or **SSPL** (Server-side Public License) — restrictive licenses discouraging commercial SaaS use without a license. Controversial.

### Dual Licensing

A project can offer two licenses: open source (GPL) and commercial. Example:

- Use under GPL v3 for free (open source terms)
- Buy a commercial license for proprietary products (typically expensive)

Examples: Qt, MySQL (originally). Creates revenue stream but requires clear governance (who can dual-license? same copyright holder?)

### CLA vs. DCO

**CLA (Contributor License Agreement)**: Contributor signs a document assigning copyright (or granting broad license) to the project. Ensures the project can relicense if needed.

Controversy: Some fear CLAs hand over rights. Projects like Linux and Kubernetes have largely replaced CLAs.

**DCO (Developer Certificate of Origin)**: Contributor adds `Signed-off-by: Name <email>` to commits, certifying they have the right to contribute. Lighter weight, no legal transfer.

Increasingly popular as a CLA replacement.

## SPDX Identifiers & License Scanning

**SPDX** is a standardized short code for each license (e.g., `MIT`, `GPL-2.0-only`, `Apache-2.0`).

```json
// package.json
{
  "license": "MIT"
}
```

```
// SPDX License List in source code
// SPDX-License-Identifier: MIT
```

**License scanning tools** (FOSSA, Black Duck, WhiteSource): Scan your dependencies for licenses, check compatibility, and flag violations. Outputs: "You depend on X packages, Y are GPL, Z have incompatible licenses."

## See Also

- **Open Source Practices** — Contributing, community, governance
- **Open Source Sustainability** — Funding models and maintainer health
- **Software Supply Chain Security** — SBOM and license as part of supply chain
- **API Design** — License explicitly in SDK/library documentation