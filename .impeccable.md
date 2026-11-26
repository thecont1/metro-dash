## Design Context

### Users
NammaMetro in Charts serves non-technical public-data consumers: Bengaluru residents, metro commuters, journalists, researchers, students, and civic observers who want to understand how the city moves. They encounter it as a lightweight public web page with no sign-up or download, often on mobile and sometimes with limited attention or connectivity. Their job is to check the latest ridership, explore patterns over time, compare commute-oriented and casual fare media, and trust what they are seeing without needing BI-tool or statistical expertise.

### Brand Personality
The brand is civic data journalism: independent, transparent, useful, and carefully crafted. In three words: **clear, curious, accountable**. The voice is plain-spoken and precise, with editorial warmth rather than institutional authority. The experience should evoke calm civic clarity, editorial curiosity, delightful pattern-spotting, and public accountability. It should feel like a thoughtful public utility built for citizens, not like an official BMRCL product or a corporate analytics suite.

### Aesthetic Direction
Refine the current light editorial direction rather than replacing it. The interface is print-inspired and restrained: a white/paper surface, black ink, fine hairlines, generous whitespace, one chart per view, and typography that mixes utilitarian UI sans-serifs with editorial serif italics. The established palette centers on Namma purple (`#5E2D8C`) and its sequential heatmap bands, with gold (`#c8a44d`) as the casual-series counterpoint and the existing fare-media colors used sparingly. Typography is anchored by Inter for body/UI, Fraunces for editorial emphasis, DM Sans for compact labels/actions, and JetBrains Mono for dates and numeric precision.

Light mode is the intended primary direction for now; do not add dark mode unless explicitly requested. Avoid the dense BI-dashboard look, generic admin UI patterns, unnecessary nested cards, decorative gradients/shadows, hype-oriented dataviz, and anything that feels like a transit authority press release. Visual interest should come from strong typography, meaningful color, careful hierarchy, and small moments of discovery—not ornament for its own sake.

### Design Principles
1. **Public evidence before decoration.** Every visual choice should improve comprehension, provenance, or trust. Source, methodology, freshness, and missing-data states are part of the interface, not footnotes.
2. **Editorial calm with moments of discovery.** Keep the reading environment quiet and legible, then use animation, annotations, tooltips, and progressive disclosure to make patterns feel satisfying to uncover.
3. **Data honesty over visual polish.** Missing values, incomplete series, percentile relativity, and methodological caveats must remain visible and understandable; never imply false precision or hide uncertainty to make the charts look cleaner.
4. **Accessible by default.** Design toward WCAG 2.2 AA. Preserve keyboard and screen-reader paths, SSR/fallback content, reduced-motion behavior, sufficient contrast, visible focus states, and colorblind-safe redundancy; hue must never be the only carrier of meaning.
5. **One clear story at a time.** Favor focused chart views, direct controls, mobile-friendly interaction, and progressive enhancement over dense dashboards or simultaneous competing visualizations.
