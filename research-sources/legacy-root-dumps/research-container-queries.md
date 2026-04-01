# @container - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@container

@container 

Baseline

Widely available

*

This feature is well established and works across many devices and browser versions. It’s been available across browsers since February 2023.

* Some parts of this feature may have varying levels of support. 

Learn more

See full compatibility

Report feedback

The @container CSS at-rule is a conditional group rule that applies styles to a containment context .
Style declarations are filtered by a condition and applied to the container if the condition is true.
The condition is evaluated when the queried container size, <style-feature> , scroll-state, or state of the applied position-try fallback (in the case of anchor-positioned containers) changes. 

The condition must specify one or both of container-name and <container-query> . 

The container-name property specifies a list of query container names, which are used to filter which containers are targeted by the @container rules.
The container features in the <container-query> are evaluated against the selected containers.
If no <container-name> is specified, the <container-query> features are evaluated against the nearest ancestor query container that has the matching container-type .
If no <container-query> is specified, named containers are selected. 

Syntax 

css 
/* With a <size-query> */
@container (width > 400px) {
h2 {
font-size: 1.5em;
}
}

/* With an optional <container-name> */
@container tall (height > 30rem) {
p {
line-height: 1.6;
}
}

/* With a <container-name> only (query is optional) */
@container sidebar {
h2 {
background: blue;
}
}

/* With a <scroll-state> */
@container scroll-state(scrollable: top) {
.back-to-top-link {
visibility: visible;
}
}

/* With an anchored query */
@container anchored(fallback: bottom) {
.infobox::before {
content: "▲";
bottom: 100%;
top: auto;
}
}

/* With a <container-name> and a <scroll-state> */
@container sticky-heading scroll-state(stuck: top) {
h2 {
background: purple;
color: white;
}
}

/* Multiple queries in a single condition */
@container (width > 400px) and style(--responsive: true) {
h2 {
font-size: 1.5em;
}
}

/* Condition list */
@container card (width > 400px), style(--responsive: true), scroll-state(stuck: top) {
h2 {
font-size: 1.5em;
}
}

Parameters 

<container-condition> 

One or both of <container-name> and <container-query> .
Styles defined in the <stylesheet> are applied if the condition is true . 

<container-name> Optional 

The name of the container that the styles will be applied to when the query evaluates to true , specified as an <ident> . 

<container-query> Optional 

A set of features that are evaluated against the query container when the size, <style-feature> , scroll-state, or applied position-try fallback of the container changes. 

Logical keywords in container queries 

Logical keywords can be used to define the container condition: 

and combines two or more conditions. 

or combines two or more conditions. 

not negates the condition. Only one 'not' condition is allowed per container query and cannot be used with the and or or keywords. 

css 
@container (width > 400px) and (height > 400px) {
/* <stylesheet> */
}

@container (width > 400px) or (height > 400px) {
/* <stylesheet> */
}

@container not (width < 400px) {
/* <stylesheet> */
}

Named containment contexts 

A containment context can be named using the container-name property. 

css 
.post {
container-name: sidebar;
container-type: inline-size;
}

The shorthand syntax for this is to use container in the form container: <name> / <type> , for example: 

css 
.post {
container: sidebar / inline-size;
}

In container queries, the container-name property is used to filter the set of containers to those with a matching query container name: 

css 
@container sidebar (width > 400px) {
/* <stylesheet> */
}

Details about usage and naming restrictions are described in the container-name page. 

Descriptors 

The <container-condition> queries include size , scroll-state , and anchored container descriptors. 

Size container descriptors 

The <container-condition> can include one or more boolean size queries, each within a set of parentheses. A size query includes a size descriptor, a value, and — depending on the descriptor — a comparison operator. The queries always measures the content box as the comparison. The syntax for including multiple conditions is the same as for @media size feature queries. 

css 
@container (min-width: 400px) {
/* … */
}
@container (orientation: landscape) and (width > 400px) {
/* … */
}
@container (15em <= block-size <= 30em) {
/* … */
}

aspect-ratio 

The aspect-ratio of the container calculated as the width to the height of the container expressed as a <ratio> value. 

block-size 

The block-size of the container expressed as a <length> value. 

height 

The height of the container expressed as a <length> value. 

inline-size 

The inline-size of the container expressed as a <length> value. 

orientation 

The orientation of the container, either landscape or portrait . 

width 

The width of the container expressed as a <length> value. 

Scroll-state container descriptors 

Scroll-state container descriptors are specified inside the <container-condition> as an argument for the scroll-state() function, for example: 

css 
@container scroll-state(scrollable: top) {
/* … */
}
@container scroll-state(scrolled: block-end) {
/* … */
}
@container scroll-state(stuck: inline-end) {
/* … */
}
@container scroll-state(snapped: both) {
/* … */
}

Supported keywords for scroll-state container descriptors include physical and flow relative values. 

scrollable 

Queries whether the container can be scrolled in the given direction via user-initiated scrolling, such as by dragging the scrollbar or using a trackpad gesture. In other words, is there overflowing content in the given direction that can be scrolled to? Valid scrollable values include the following keywords: 

none 

The container is not a scroll container or otherwise cannot be scrolled in any direction. 

top 

The container can be scrolled towards its top edge. 

right 

The container can be scrolled towards its right-hand edge. 

bottom 

The container can be scrolled towards its bottom edge. 

left 

The container can be scrolled towards its left-hand edge. 

x 

The container can be scrolled horizontally towards either or both of its left-hand or right-hand edges. 

y 

The container can be scrolled vertically towards either or both of its top or bottom edges. 

block-start 

The container can be scrolled towards its block-start edge. 

block-end 

The container can be scrolled towards its block-end edge. 

inline-start 

The container can be scrolled towards its inline-start edge. 

inline-end 

The container can be scrolled towards its inline-end edge. 

block 

The container can be scrolled in its block direction towards either or both of its block-start or block-end edges. 

inline 

The container can be scrolled in its inline direction towards either or both of its inline-start and inline-end edges. 

If the test passes, the rules inside the @container block are applied to descendants of the scroll container. 

To evaluate whether a container is scrollable, without being concerned about the direction, use the none value with the not operator: 

css 
@container not scroll-state(scrollable: none) {
/* … */
}

scrolled 

Queries whether the container was most recently scrolled in a specified direction. Valid scrolled values include the following keywords: 

none 

The container is not a scroll container or otherwise has not previously been scrolled in any direction. 

top 

The container was most recently scrolled towards its top edge. 

right 

The container was most recently scrolled towards its right-hand edge. 

bottom 

The container was most recently scrolled towards its bottom edge. 

left 

The container was most recently scrolled towards its left-hand edge. 

x 

The container was most recently scrolled towards either its left-hand or right-hand edges. 

y 

The container was most recently scrolled towards either its top or bottom edges. 

block-start 

The container was most recently scrolled towards its block-start edge. 

block-end 

The container was most recently scrolled towards its block-end edge. 

inline-start 

The container was most recently scrolled towards its inline-start edge. 

inline-end 

The container was most recently scrolled towards its inline-end edge. 

block 

The container was most recently scrolled towards either its block-start or block-end edges. 

inline 

The container was most recently scrolled towards either its inline-start or inline-end edges. 

If the test returns true, the rules nested in the @container block are applied to the descendants of the scroll container. 

To evaluate whether a container has recently been scrolled, without being concerned about the direction, use the none value with the not operator: 

css 
@container not scroll-state(scrolled: none) {
/* … */
}

snapped 

Queries whether the container is going to be snapped to a scroll snap container ancestor along the given axis. Valid snapped values include the following keywords: 

none 

The container is not a scroll snap target for its ancestor scroll container. When implementing a snapped: none query, containers that are snap targets for the scroll container will not have the @container styles applied, whereas non-snap targets will have the styles applied. 

x 

The container is a horizontal scroll snap target for its ancestor scroll container, that is, it is snapping horizontally to its ancestor. 

y 

The container is a vertical scroll snap target for its ancestor scroll container, that is, it is snapping vertically to its ancestor. 

block 

The container is a block-axis scroll snap target for its ancestor scroll container, that is, it is snapping to its ancestor in the block direction. 

inline 

The container is an inline-axis scroll snap target for its ancestor scroll container, that is, it is snapping to its ancestor in the inline direction. 

both 

The container is both a horizontal and vertical scroll snap target for its ancestor scroll container and is snapping to its ancestor in both directions. The container won't match if it is only snapping to its ancestor along the horizontal or vertical axis. It needs to be both. 

To evaluate a container with a non- none snapped scroll-state query, it must be a container with a scroll container ancestor having a scroll-snap-type value other than none . A snapped: none query will match even when there is no scroll container ancestor. 

Evaluations occur when scrollsnapchanging events fire on the scroll snap container. If the test passes, the rules inside the @container block are applied to descendants of the container. 

To evaluate whether a container is a snap target, without being concerned about the direction, use the none value with the not operator: 

css 
@container not scroll-state(snapped: none) {
/* … */
}

stuck 

Queries whether a container with a position value of sticky is stuck to an edge of its scrolling container ancestor. Valid stuck values include the following keywords: 

none 

The container is not stuck to any edges of its container. Note that none queries will match even if the container does not have position: sticky set on it. 

top 

The container is stuck to the top edge of its container. 

right 

The container is stuck to the right-hand edge of its container. 

bottom 

The container is stuck to the bottom edge of its container. 

left 

The container is stuck to the left-hand edge of its container. 

block-start 

The container is stuck to the block-start edge of its container. 

block-end 

The container is stuck to the block-end edge of its container. 

inline-start 

The container is stuck to the inline-start edge of its container. 

inline-end 

The container is stuck to the inline-end edge of its container. 

To evaluate a container with a non- none stuck scroll-state query, it must have position: sticky set on it, and be inside a scroll container. If the test passes, the rules inside the @container block are applied to descendants of the position: sticky container. 

It is possible for two values from adjacent axes to match at the same time: 

css 
@container scroll-state((stuck: top) and (stuck: left)) {
/* … */
}

However, two values from opposite edges will never match at the same time: 

css 
@container scroll-state((stuck: left) and (stuck: right)) {
/* … */
}

To evaluate whether a container is stuck, without being concerned about the direction, use the none value with the not operator: 

css 
@container not scroll-state(stuck: none) {
/* … */
}

Anchored container descriptors 

Anchored container descriptors are specified inside the <container-condition> as an argument for the anchored() function, for example: 

css 
@container anchored(fallback: top) {
/* … */
}
@container anchored(fallback: flip-block flip-inline) {
/* … */
}
@container anchored(fallback: --custom-fallback) {
/* … */
}

fallback 

Queries whether a specific position-try fallback is currently active on an anchor-positioned container, as specified via the position-try-fallbacks property. Valid fallback values include any component value that is valid for inclusion in a position-try-fallbacks property value. 

If the fallback value named in the test is currently active on the anchor-positioned container, the test passes, and the rules inside the @container block are applied to descendants of the anchor-positioned container. 

Formal syntax 

@container = 
@container <container-condition> # { <block-contents> } 

"><container-condition> = 
[ <container-name> ? <container-query> ? ] ! 

"><container-name> = 
<custom-ident> 

"><container-query> = 
not <query-in-parens> | 
<query-in-parens> [ [ and <query-in-parens> ] * | [ or <query-in-parens> ] * ] 

"><query-in-parens> = 
( <container-query> ) | 
( <size-feature> ) | 
style( <style-query> ) | 
scroll-state( <scroll-state-query> ) | 
<general-enclosed> 

See also 

Using container queries 

Using container size and style queries 

Using container scroll-state queries 

Using anchored container queries 

container-name 

container-type 

contain 

content-visibility 

CSS containment module 

CSS at-rule functions 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Mar 24, 2026 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# container-type - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/container-type

container-type 

Baseline

Widely available

*

This feature is well established and works across many devices and browser versions. It’s been available across browsers since February 2023.

* Some parts of this feature may have varying levels of support. 

Learn more

See full compatibility

Report feedback

An element can be established as a query container using the container-type CSS property. container-type is used to define the type of container context used in a container query. The available container contexts are: 

Size : Enable selectively applying CSS rules to a container's children based on a general size or inline size condition such as a maximum or minimum dimension, aspect ratio, or orientation. 

Scroll-state : Enable selectively applying CSS rules to a container's children based on a scroll-state condition such as whether the container is a scroll container that is partially scrolled or whether the container is a snap target that is going to be snapped to its scroll snap container. 

Anchored : Enable selectively applying CSS rules to a container's children based on whether the container is anchor-positioned and has a position-try fallback option applied to it. 

Syntax 

css 
/* Keyword values */
container-type: normal;
container-type: size;
container-type: inline-size;
container-type: scroll-state;
container-type: anchored;

/* Two values */
container-type: size scroll-state;

/* Global Values */
container-type: inherit;
container-type: initial;
container-type: revert;
container-type: revert-layer;
container-type: unset;

Values 

The container-type property can take a single value from the list below, or two values — one must be scroll-state and the other can be inline-size or size . In other words, an element can be established as a size query container, a scroll-state query container, both, or neither. 

anchored 

Establishes a query container for anchored container queries on the container. In this case, the size of the element is not computed in isolation; no containment is applied. 

inline-size 

Establishes a query container for dimensional queries on the inline axis of the container.
Applies style and inline-size containment to the element. The inline size of the element can be computed in isolation , ignoring the child elements (see Using CSS containment ). 

normal 

Default value. The element is not a query container for any container size queries, but remains a query container for container style queries . 

scroll-state 

Establishes a query container for scroll-state queries on the container. In this case, the size of the element is not computed in isolation; no containment is applied. 

size 

Establishes a query container for container size queries in both the inline and block dimensions.
Applies style and size containment to the element. Size containment is applied to the element in both the inline and block directions. The size of the element can be computed in isolation, ignoring the child elements. 

Formal definition 

Initial value normal 
Applies to all elements 
Inherited no 
Computed value as specified 
Animation type a CSS data type are interpolated on each of their red, green, blue components, each handled as a real, floating-point number. Note that interpolation of colors happens in the alpha-premultiplied sRGBA color space to prevent unexpected grey colors to appear.">color 

Formal syntax 

container-type = 
normal | 
[ [ size | inline-size ] || scroll-state ] 

Description 

Container queries allow you to selectively apply styles inside a container based on conditional queries performed on the container. The @container at-rule is used to specify the tests performed on a container, and the rules that will apply to the container's contents if the query returns true . 

The container query tests are only performed on elements with a container-type property, which defines the elements as a size, scroll-state, or anchored query container, or a combination thereof. 

Container size queries 

Container size queries allow you to selectively apply CSS rules to a container's descendants based on a size condition such as a maximum or minimum dimension, aspect ratio, or orientation. 

Size containers additionally have size containment applied to them — this turns off the ability of an element to get size information from its contents, which is important for container queries to avoid infinite loops. If this were not the case, a CSS rule inside a container query could change the content size, which in turn could make the query evaluate to false and change the parent element's size, which in turn could change the content size and flip the query back to true, and so on. This sequence would then repeat itself in an endless loop. 

The container size has to be set by context, such as block-level elements that stretch to the full width of their parent, or explicitly defined. If a contextual or explicit size is not available, elements with size containment will collapse. 

Container scroll-state queries 

Container scroll-state queries allow you to selectively apply CSS rules to a container's children based on a scroll-state condition such as: 

Whether the container's contents are partially scrolled. 

Whether the container is a snap target that is going to be snapped to a scroll snap container. 

Whether the container is positioned via position: sticky and stuck to a boundary of a scrolling container . 

In the first case, the queried container is the scroll container itself. In the other two cases the queried container is an element that is affected by the scroll position of an ancestor scroll container. 

Anchored container queries 

Anchored container queries allow you to selectively apply CSS rules to the descendants of an anchor-positioned container when it has a position-try fallback active on it, as specified via the position-try-fallbacks property. 

For example, you might have an anchor-positioned tooltip element that is positioned above its anchor by default via a position-area value of top , but has a position-try-fallbacks value of flip-block specified. This will cause the tooltip to flip in the block direction to the bottom of its anchor when it starts to overflow the top of the viewport. If we set container-type: anchored on it, we can detect when the position-try fallback is applied via a @container at-rule and apply CSS as a result. 

css 
.tooltip {
position: absolute;
position-anchor: --myAnchor;
position-area: top;
position-try-fallbacks: flip-block;
container-type: anchored;
}

Examples 

Establishing inline size containment 

Given the following HTML example which is a card component with an image, a title, and some text: 

html 
<div class="container">
<div class="card">
<h3>Normal card</h3>
<div class="content">
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</div>
</div>
</div>

<div class="container wide">
<div class="card">
<h3>Wider card</h3>
<div class="content">
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</div>
</div>
</div>

To create a container context, add the container-type property to an element.
The following uses the inline-size value to create a containment context for the inline axis of the container: 

css 
.container {
container-type: inline-size;
width: 300px;
height: 120px;
}

.wide {
width: 500px;
}

h3 {
height: 2rem;
margin: 0.5rem;
}

.card {
height: 100%;
}

.content {
background-color: wheat;
height: 100%;
}

.container {
margin: 1rem;
border: 2px dashed red;
overflow: hidden;
}

Writing a container query via the @container at-rule will apply styles to the elements of the container when it is wider than 400px: 

css 
@container (width > 400px) {
.card {
display: grid;
grid-template-columns: 1fr 2fr;
}
}

Specifications 

Specification 

CSS Conditional Rules Module Level 5 
# container-type 

CSS Anchor Positioning Module Level 2 
# container-type-anchored 

Browser compatibility 

See also 

CSS container queries 

Using container size and style queries 

Using container scroll-state queries 

Using anchored container queries 

@container at-rule 

CSS container shorthand property 

CSS container-name property 

CSS content-visibility property 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Mar 24, 2026 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# container-name - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/container-name

container-name 

Baseline

Widely available

This feature is well established and works across many devices and browser versions. It’s been available across browsers since February 2023.

Learn more

See full compatibility

Report feedback

The container-name CSS property specifies a list of query container names used by the @container at-rule in a container query .
A container query will apply styles to elements based on the size or scroll-state of the nearest ancestor with a containment context.
When a containment context is given a name, it can be specifically targeted using the @container at-rule instead of the nearest ancestor with containment. 

Syntax 

css 
container-name: none;

/* A single name */
container-name: my-layout;

/* Multiple names */
container-name: my-page-layout my-component-library;

/* Global Values */
container-name: inherit;
container-name: initial;
container-name: revert;
container-name: revert-layer;
container-name: unset;

Values 

none 

Default value. The query container has no name. 

<custom-ident> 

A case-sensitive string that is used to identify the container.
The following conditions apply: 

The name must not equal or , and , not , or default . 

The name value must not be in quotes. 

The dashed ident intended to denote author-defined identifiers (e.g., --container-name ) is permitted. 

A list of multiple names separated by a space is allowed. 

Formal definition 

Initial value none 
Applies to all elements 
Inherited no 
Computed value none or an ordered list of identifiers 
Animation type Not animatable 

Formal syntax 

container-name = 
none | 
<custom-ident> + 

Examples 

Using a container name 

Given the following HTML example which is a card component with a title and some text: 

html 
<div class="card">
<div class="post-meta">
<h2>Card title</h2>
<p>My post details.</p>
</div>
<div class="post-excerpt">
<p>
A preview of my <a href="https://example.com">blog post</a> about cats.
</p>
</div>
</div>

To create a containment context, add the container-type property to an element in CSS.
The following example creates two containment contexts, one for the card meta information and one for the post excerpt: 

Note: 
A shorthand syntax for these declarations are described in the container page. 

css 
.post-meta {
container-type: inline-size;
}

.post-excerpt {
container-type: inline-size;
container-name: excerpt;
}

Writing a container query via the @container at-rule will apply styles to the elements of the container when the query evaluates to true.
The following example has two container queries, one that will apply only to the contents of the .post-excerpt element and one that will apply to both the .post-meta and .post-excerpt contents: 

css 
@container excerpt (width >= 400px) {
p {
visibility: hidden;
}
}

@container (width >= 400px) {
p {
font-size: 2rem;
}
}

For more information on writing container queries, see the CSS Container Queries page. 

Using multiple container names 

You can also provide multiple names to a container context separated by a space: 

css 
.post-meta {
container-type: inline-size;
container-name: meta card;
}

This will allow you to target the container using either name in the @container at-rule.
This is useful if you want to target the same container with multiple container queries where either condition could be true: 

css 
@container meta (width <= 500px) {
p {
visibility: hidden;
}
}

@container card (width <= 200px) {
h2 {
font-size: 1.5em;
}
}

Specifications 

Specification 

CSS Conditional Rules Module Level 5 
# container-name 

Browser compatibility 

See also 

CSS container queries 

Using container size and style queries 

Using container scroll-state queries 

@container at-rule 

CSS container shorthand property 

CSS container-type property 

CSS content-visibility property 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Nov 7, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content