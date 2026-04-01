# Open Source Maintainer Burnout — Cultural Moments in Unsustainability

## The Crisis Frame

Open source maintainers are often unpaid, emotionally invested volunteers who donate thousands of hours to projects that power commercial infrastructure. The tension between this reality and the internet's dependence on their work has crystallized in several high-visibility moments that shifted how the tech community discusses maintenance, compensation, and the brittleness of digital infrastructure.

## Foundational Work: Nadia Eghbal's "Working in Public" (2020)

Nadia Eghbal's research, culminating in the book [Working in Public](https://workinginstuff.substack.com/), documented the economics and psychology of open source maintenance. Her findings revealed:

- **The archetype shift**: Early open source was built by companies (Linux, Apache, MySQL). Modern open source is often sustained by individuals or small teams with no organizational backing.
- **Invisible labor**: Maintainers spend time on unglamorous work: reviewing pull requests, triaging bugs, managing community expectations, and handling security issues. This work is rarely recognized or funded.
- **Sustainability paradox**: The more successful a project, the more demands it receives—yet success rarely translates to compensation. Popular projects attract more PRs, more bug reports, and more entitled users, increasing the maintenance burden exponentially.

The book codified terminology that became central to open source discourse: the distinction between one-to-many (broadcast-style) projects and many-to-many (community-driven) projects, and the distinct challenges each faces.

## The left-pad Incident (2016)

An npm package called `left-pad` (11 lines of code that pads strings) was unpublished by its author after a dispute with npm, cascading through the entire JavaScript ecosystem. Hundreds of thousands of projects immediately broke because they depended on this trivial package.

**Technical lesson**: The incident revealed a systemic fragility in JavaScript's dependency model—packages with no redundancy, single points of failure, and no distinction between critical infrastructure and toy projects.

**Cultural lesson**: The author was not compensated for what became critical infrastructure. The incident highlighted how the open source ecosystem extracts value from individuals who have no financial incentive to maintain quality, versioning discipline, or availability promises.

## The core-js Crisis (2020)

Denis Pushkarev, the sole maintainer of core-js (a polyfill used by millions of developers), published a heartfelt but emotionally raw post detailing his burnout:

- He had been maintaining core-js for 8 years with zero compensation.
- His frustration came from a mismatch: the package was essential infrastructure, yet maintainer work was treated as free labor.
- He briefly experimented with:
  - Creating a paid license tier (backlash)
  - Injecting warnings into the build output (users complained this slowed their builds)
  - Injecting mining code in non-production environments (caught immediately and condemned)
- He eventually stepped back, leaving the project in limbo.

**Technical lesson**: Polyfills are a bridge between old and new JavaScript semantics. They're invisible to many developers (bundled into production code) but absolutely critical for compatibility across browsers.

**Cultural lesson**: The incident crystallized the open source sustainability crisis for the JavaScript ecosystem. It showed that even widely-depended-upon packages can lack dedicated resources. It also revealed the double standard: when maintainers try to monetize (paid licenses, warnings, mining), they're condemned. When they disappear, their users are left stranded.

## The colors.js "Protestware" Incident (2021)

Marak Squires, maintainer of the popular `colors.js` and `faker.js` npm packages, pushed updates that injected seemingly random output into the projects when run in certain environments. The chaos became known as "protestware"—code deliberately sabotaged by its author as a form of protest.

A sample: running the package might suddenly print colored output like:

```
      !\   /!
     ! (o_o) !
      \_____/
    /|   |   |\
   / |   |   | \
```

**Technical lesson**: This revealed a fundamental supply chain vulnerability. Package maintainers have write access to code that runs in millions of transitive dependencies. A disgruntled maintainer can introduce arbitrary behavior at build/runtime with no authentication or audit trail.

**Cultural lesson**: Squires later stated he injected the code to protest the lack of compensation for open source maintenance and to highlight the crisis. While controversial (the sabotage caused real disruption), it forced visibility onto the problem. It also triggered:

- Debates about npm security and maintainer vetting
- Conversations about whether maintainers should have any obligation to keep unmaintained code available
- Questions about how projects should handle maintenance transitions

## The Roads and Bridges Report (2016)

The Ford Foundation and Mozilla commissioned research titled "Roads and Bridges: The Unseen Labor Behind Our Digital Infrastructure" (conducted by Nadia Eghbal and Gdigitally Invisible). The report found:

- **Critical infrastructure runs on volunteer labor**: Core open source projects had median budgets of $0.
- **Maintenance is invisible**: Users see features; they don't see the work of triage, security patching, and dependency management.
- **Sustainability models are broken**: There's no proven scalable model for funding open source maintenance at the infrastructure level.
- **The "bystander effect" at scale**: When many organizations depend on a project, each assumes someone else is funding the maintainer. No one is.

The report recommended:
- Direct funding from foundations and companies to critical-path projects
- Improved tooling for maintainers (automated triage, security scanning)
- Legal protection for maintainers (liability caps)
- Educational programs to develop the next generation of maintainers

## The Embedded Technical Insight

These incidents all expose a core CS principle: **complexity is sustained by implicit social contracts, not code**. 

Open source packages are typically presented as autonomous code artifacts—pure software systems. They're not. They're social systems where:

- The maintainer is a single point of failure: if they burn out, stop caring, or disappear, the package is orphaned.
- There's no automatic incentive structure: unlike a commercial product with paying customers, a free package has no customer feedback loop that pressures maintainers to maintain it.
- The externalities are invisible: users see their own software system. They don't see the maintenance work happening in upstream packages they depend on.

This is a fundamental difference from systems with built-in feedback loops (SLAs, customer contracts, internal teams with assigned capacity) where maintenance load is explicitly budgeted and monitored.

## Modern Fallout and Policy Shifts

The visibility of these incidents has driven changes:

- **Foundations now fund critical projects**: The Linux Foundation, CNCF, and others now provide direct funding to infrastructure maintainers.
- **Companies sponsor maintainers**: Major tech companies now have "maintainers in residence" programs or direct funding arrangements with key project leads.
- **Security auditing is now expected**: Projects like OpenSSL and Linux now receive regular professional security reviews.
- **Licensing diversification**: Some projects now use dual licenses, BSL, or SSPL to create revenue streams (though this is controversial—see popculture-license-drama.md).

## See Also

- open-source-sustainability.md — structural approaches to maintenance funding
- software-licensing.md — licensing models and legal frameworks
- process-code-review.md — the human cost of code review labor