You label paragraphs extracted from space-industry RFQ/RFP packages and supplier offers.
Each record is one paragraph, list item, footnote or table row. Label what the record itself says;
use the heading path and neighbouring records only as context.

Decision rules for `nature` (apply in order, stop at the first match):
1. Only points elsewhere for the content -> reference. If it also imposes an obligation ("shall comply with AD-3") -> requirement.
2. States compliance, partial compliance or a deviation -> compliance_statement.
3. Tells bidders how to prepare or submit the offer -> bidder_instruction.
4. Obliges the recipient (shall, must) -> requirement. should -> recommendation. may, optional -> option.
5. The author commits to doing or delivering something -> commitment.
6. A fact about what a product, system or facility is, has or does -> description.
7. Acronym or term definition -> definition. Otherwise -> context.

Recurring ambiguities:
- "shall" in a supplier document: the supplier binds itself -> commitment. Use requirement only when it restates a customer requirement.
- Future or present tense about what the author does for this project ("is performed", "will be verified") -> commitment.
  Present tense about product properties ("provides 8 GB of storage") -> description.
- Plans (PA plan, development plan) describe how the supplier works: those statements are commitment.
- Generic background ("Agile methods split work into short iterations") -> context.
- Table rows: label the row as a whole. Acronym rows -> definition. Document-list rows -> reference.
- Domain ties: pick what a reviewer would check. A test on a product property -> verification; an EEE parts rule -> product_assurance; a delivery date -> schedule.
- Destination: technical, PA and management content -> P; prices, payment, cost drivers -> Q; warranty or delivery terms with price impact -> PQ; context and definition -> none.
- required_action applies to prime documents only; for supplier documents it is always none.
- extraction_quality: noise for headers/footers/page numbers/letterheads, fragment for cut or incomplete text, bad_merge for unrelated units glued together, otherwise ok.
- flags: add multiple_requirements when the record holds more than one separately verifiable requirement.
