# Falsehoods Programmers Believe: The Series

## Overview

"Falsehoods Programmers Believe About X" is a crowdsourced series of articles documenting edge cases and counterintuitive facts in ostensibly simple domains (names, time, addresses, phone numbers, email). Each started as a sarcastic blog post by Patrick McKenzie and evolved into community-driven catalogs of hard-won knowledge. They're essential reading for anyone designing databases, validation rules, or internationalization systems.

---

## The Series Format

Each article lists 20–30+ "falsehoods" (assumptions) and explains why they're wrong with real-world examples. Structure:

```
Falsehood: [Simple-sounding assumption]
Reality: [Edge case or exception, usually with international/cultural/technical examples]
```

For instance:

**Falsehood**: "A person's name fits in 255 characters."  
**Reality**: Some names are legitimately longer. Mongolian names, for example, can exceed 255 characters when including generational titles and cultural context.

---

## Falsehoods about Names

**Original author**: Patrick McKenzie (2010)  
**Core premise**: Names are not as simple as `FirstName LastName`.

### Key Falsehoods

1. **"All people have names"** — Some cultures don't use names; status or kinship is the identifier. Some people are known only by epithets.

2. **"Names are persistent"** — Many cultures change names for marriage, religious conversion, or social status. Names are not immutable identifiers.

3. **"There is a standardized way to order names"** — Western cultures use (Given, Family). Chinese uses (Family, Given). Some cultures use (Given, Patronymic, Family). Some don't have fixed order.

4. **"A name belongs to a person"** — Companies, ships, places also have names. A name field must handle non-person entities.

5. **"Only Latin characters are valid"** — Names use Cyrillic, Arabic, CJK, Hangul, etc. Supporting any name requires Unicode.

6. **"A person has a first and last name"** — Some people have multiple given names, multiple family surnames (Portuguese João da Silva), or no family name at all (many Indonesians).

7. **"Name fields should be mandatory"** — Some systems omit names entirely. A system should handle it gracefully.

8. **"Names are unique identifiers"** — Many people share names. Names are descriptive, not identifying. Use a separate ID field.

9. **"Separating first/last name is always correct"** — For non-Western cultures, this separation is arbitrary and data-losing. Store full name, don't parse.

10. **"Nicknames are rare"** — In many cultures and informal systems, nicknames are primary identifiers. Bob is as valid an ID as Robert.

### Design Lesson

**Never parse a name into components.** Store the full name as the user provides it. If you must separate (form field usability), do so at input but retain the full name. Parsing breaks for:
- Vietnamese names (no surname, generational markers)
- Icelandic names (patronymics, not family names)
- Arabic names (multiple particles: "ibn," "al," "abd")

User provided: "José María García López"  
Incorrect parse: First="José María", Last="García López"  
Correct: Full name="José María García López" (possibly with user-chosen display format)

---

## Falsehoods about Time

**Original author**: Evan Carroll  
**Core premise**: Time is not a simple `TIMESTAMP` field.

### Key Falsehoods

1. **"There is only one 'now'"** — Different systems have different clocks. A server's "now" may differ from a client's. Relativity and network delays matter.

2. **"Dates have a defined ordering"** — Different calendars (Gregorian, Hebrew, Islamic, Buddhist) have different epoch dates. Comparison is context-dependent.

3. **"Weeks start on Monday"** — In the US, weeks start on Sunday. In the Middle East, Saturday. In ISO 8601, Monday. The boundary is cultural.

4. **"Three-digit time zones are unambiguous"** — Some regions adjust DST on different dates, creating overlaps. A time without DST info is ambiguous.

5. **"Daylight saving time is straightforward"** — Some regions don't observe DST. Some observe it on non-standard dates. Some observe it for non-standard durations. Handling requires a DST database (IANA TZDB or equivalent).

6. **"You can store time as a Unix timestamp"** — Unix timestamp loses DST information. "1:30 AM" occurs twice on DST transition day. Which one did the user mean?

7. **"Leap seconds don't happen"** — LEAP SECONDS DO HAPPEN (rarely, but they do). Unix timestamp math breaks on leap seconds.

8. **"Months always have the same number of days"** — September has 30 days, except when it doesn't in some calendar systems. February has 28 or 29. Calculation must account for the specific year.

9. **"Duration is just two timestamps"** — "1 hour" is not always 3600 seconds because of DST transitions. A "1-hour meeting" during DST shift is 59 or 61 minutes.

10. **"Time zones have a fixed UTC offset"** — Historical time zones change. In 2000, the Samoa timezone jumped 23 hours forward. A stored timestamp with a timezone name is ambiguous.

### Design Lesson

**Store timestamps in UTC and store timezone separately.** When displaying, convert using the user's timezone and TZDB library. For durations that cross DST boundaries, store start/end times separately or use a duration library that handles DST.

```python
# WRONG
user_time = "1:30 AM EST"  # Ambiguous if this is during DST transition

# CORRECT
user_time_utc = datetime(2024, 3, 10, 6, 30, tzinfo=UTC)  # Explicit UTC
user_timezone = "America/New_York"  # Separate timezone name
```

---

## Falsehoods about Addresses

**Original author**: Khalid Abuhakmeh (expanded community effort)  
**Core premise**: Addresses don't fit the `Street, City, State, ZIP` format globally.

### Key Falsehoods

1. **"There is a standard address format"** — UK: Street, City, Postcode. Germany: Street/Number, PLZ, City. Japan: Prefecture, City, District, Block, Lot. No global standard.

2. **"A postal code is five digits"** — US: 5 or 9 (ZIP+4). UK: 7 characters with space. Canada: 6 alphanumeric. Length varies; format varies.

3. **"A country has one address format"** — India uses PIN code. China uses postal code plus area code. Some countries have no postal code system.

4. **"Addresses fit in a database row"** — Some Thai addresses include Buddhist temple names. Some Brazilian addresses reference landmarks ("near the big tree on Rua X"). Unstructured address is sometimes necessary.

5. **"Street name comes before number"** — Sweden: Street Number Street Name ("3 Vägen"). France: same. USA: Number Street ("123 Main St"). No universal order.

6. **"Cities have unique identifiers"** — Many countries have multiple towns with the same name. Address must include region/province to disambiguate.

7. **"An address maps to one location"** — Buildings with multiple addresses, street aliases, and historical address renaming create ambiguity.

8. **"Zip code implies a city"** — In the US, a ZIP code can span multiple cities or split a city.

9. **"A country is a state"** — Overseas territories, special administrative regions (Hong Kong, Macau) have their own address systems despite being politically part of another country.

10. **"Addresses are stable"** — Countries rename cities (Ho Chi Minh City was Saigon). Streets get renumbered. Postal codes change. Address history must be retained.

### Design Lesson

**Don't validate addresses strictly.** Accept freeform text with optional structure. For delivery, use the postal service's address validation API (USPS, Royal Mail, etc.). Never assume a ZIP code maps to exactly one city, state, or country.

---

## Falsehoods about Phone Numbers

**Original author**: Community effort via product research  
**Core premise**: Phone numbers are not `+1-NNN-NNN-NNNN`.

### Key Falsehoods

1. **"Phone numbers are always digits"** — Some notations include letters (alphanumeric shortcodes, shared numbers like 1-800-FLOWERS).

2. **"Every country has one format"** — Germany: +49 30 1234 5678. Japan: +81 3-1234-5678. Format and length vary.

3. **"Phone numbers are unique"** — Country code + phone number should be unique, but enforcement is regional. Duplicate numbers can exist across countries.

4. **"A phone number identifies a person"** — A number may be shared (office line), temporary (prison phone), or not belong to the subscriber (burner phones, VOIP, alias numbers).

5. **"Phone formats are consistent within a country"** — The US has NPA-NXX-XXXX (area code matters), but some numbers are restricted to specific regions. Tokyo and Osaka have different rules.

6. **"Extensions are separate from the number"** — Some services require "extension" to reach a specific person, making it part of the identifier.

7. **"There are always area codes"** — Mobile numbers in many countries don't have area codes. VoIP does not follow regional numbering.

8. **"Leading zeros are never significant"** — Germany uses 0 for domestic long-distance (+49 30 becomes 030). France: 0 is necessary domestically but omitted internationally. Stripping leading zeros breaks formatting.

9. **"All phone numbers are human-readable"** — Some shortcodes (e.g., 911 emergencies) are context-specific. Machine-generated numbers (TWILIO Flex) don't follow traditional format.

10. **"A phone number has only one purpose"** — The same number might route to voice, SMS, and fax differently. Caller ID may display differently than the actual number.

### Design Lesson

**Store phone numbers in E.164 format** (international standard: `+[country code][subscriber number]`). For display, use a library like `libphonenumber` to format for the user's locale. Never assume format or uniqueness without a service to validate.

---

## Falsehoods about Email

**Original author**: Community effort, partially documented by Ben Ward  
**Core premise**: Email is not `username@domain`.

### Key Falsehoods

1. **"An email address is a valid username"** — Emails are identifiers for messaging, not account usernames. A system should have separate ID and email.

2. **"Email local parts are case-insensitive"** — Per RFC 5321, the local part (before @) should be treated case-sensitively by the receiver. In practice, most providers treat it as case-insensitive, but it's not guaranteed.

3. **"There's only one at sign"** — Technically, `user+tag@domain` is valid. Some systems only extract the first @, breaking parsing.

4. **"The domain must exist"** — A domain can receive email without an MX record (mail defaults to A record). Validating MX alone is insufficient.

5. **"Email is unique"** — Multiple people can have access to one email. Email is not a unique identifier; it's a contact method.

6. **"Sub-addressing isn't real"** — `user+tag@domain` is a real, standardized email address (RFC 5233). Some systems treat `+tag` as invalid character.

7. **"Emails are validated at signup"** — Validation confirms syntax, not that the user owns it. Confirm ownership via email verification link.

8. **"There is one valid email per person"** — A person may have multiple email addresses. Email changes. An address can be abandoned.

9. **"Email is instant"** — Email is best-effort and asynchronous. A sent email may be delayed hours or lost. Never assume delivery.

10. **"Emails don't change"** — A user can delete an email account and have another user claim it later (recycled addresses). Don't use email as permanent identifier.

### Design Lesson

**Treat email as a contact method, not an identifier.** For persistent identity, use a separate UUID or user account ID. Validate email syntax loosely; validate ownership via confirmation link sent to the address. Expect users to use email address+tags for filtering.

---

## What These Series Teach

### 1. **Complexity Hidden by Assumption**
Most programmers assume `name`, `time`, `address`, `phone`, `email` are simple types. Reality is complex. These series reveal hidden complexity early.

### 2. **International Conventions Vary**
Western/English-speaker defaults don't generalize. Names, dates, times, addresses, and phone numbers follow different conventions globally. All input must support internationalization.

### 3. **Historical and Legacy Systems Don't Comply**
Real data is messy. Past systems made assumptions that broke. Addresses still reference demolished landmarks. Phone numbers still include non-standard formats. Robust systems must handle legacy data.

### 4. **Validation vs. Flexibility**
Strict validation (regex, format checking) catches errors but rejects valid data. Flexible approach (accept freeform, validate externally) accommodates edge cases but risks garbage data. The tradeoff depends on context.

### 5. **Identity vs. Attributes**
Names, emails, andphone numbers look like identifiers but are actually attributes. True identifiers (UUIDs, user IDs) don't change. Conflating attributes with identity causes bugs when people's names change, emails are recycled, or phone numbers are reassigned.

---

## See Also

- **database-schema-design.md** — Handling polymorphic data, nullable fields, text search
- **internationalization-localization.md** — Unicode, locale conventions, text handling
- **api-design.md** — Input validation, JSON schema, error responses
- **security-input-validation.md** — Why strict validation can be both a safety and accessibility problem
- **data-quality-master-data.md** — Managing reference data, deduplication, reconciliation