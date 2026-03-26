# content-visibility - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/content-visibility

content-visibility 

Baseline

2024

*

Newly available

Since September 2024, this feature works across the latest devices and browser versions. This feature might not work in older devices or browsers.

* Some parts of this feature may have varying levels of support. 

Learn more

See full compatibility

Report feedback

The content-visibility CSS property controls whether or not an element renders its contents at all, along with forcing a strong set of containments, allowing user agents to potentially omit large swathes of layout and rendering work until it becomes needed. It enables the user agent to skip an element's rendering work (including layout and painting) until it is needed — which makes the initial page load much faster. 

Note: 
The contentvisibilityautostatechange event fires on any element with content-visibility: auto set on it when its rendering work starts or stops being skipped. This provides a convenient way for an app's code to start or stop rendering processes (e.g., drawing on a <canvas> ) when they are not needed, thereby conserving processing power. 

Try it 

content-visibility: visible;

content-visibility: hidden;

<section class="default-example" id="default-example">
<div class="container" id="example-element">
<div class="child">
<span>This is an inner div</span>
</div>
</div>
</section>

.container {
width: 140px;
height: 140px;
border: 3px solid rgb(64 28 163);
background-color: rgb(135 136 184);
display: flex;
align-items: center;
justify-content: center;
}

.child {
border: 3px solid rgb(64 28 163);
background-color: wheat;
color: black;
width: 80%;
height: 80%;
display: flex;
align-items: center;
justify-content: center;
}

Syntax 

css 
/* Keyword values */
content-visibility: visible;
content-visibility: hidden;
content-visibility: auto;

/* Global values */
content-visibility: inherit;
content-visibility: initial;
content-visibility: revert;
content-visibility: revert-layer;
content-visibility: unset;

Values 

visible 

No effect. The element's contents are laid out and rendered as normal. This is the default value. 

hidden 

The element skips its contents . The skipped contents must not be accessible to user-agent features, such as find-in-page, tab-order navigation, etc., nor be selectable or focusable. This is similar to giving the contents display: none . 

auto 

The element turns on layout containment, style containment, and paint containment. If the element is not relevant to the user , it also skips its contents. Unlike hidden, the skipped contents must still be available as normal to user-agent features such as find-in-page, tab order navigation, etc., and must be focusable and selectable as normal. 

Description 

Animating and transitioning content-visibility 

Supporting browsers animate/transition content-visibility with a variation on the discrete animation type . 

Discrete animation generally means that the property will flip between two values 50% of the way through the animation. In the case of content-visibility , however, the browser will flip between the two values to show the animated content for the entire animation duration. So, for example: 

When animating content-visibility from hidden to visible , the value will flip to visible at 0% of the animation duration so it is visible throughout. 

When animating content-visibility from visible to hidden , the value will flip to hidden at 100% of the animation duration so it is visible throughout. 

This behavior is useful for creating entry/exit animations where you want to, for example, remove some content from the DOM with content-visibility: hidden , but you want a smooth transition (such as a fade-out) rather than it disappearing immediately. 

When animating content-visibility with CSS transitions , transition-behavior: allow-discrete needs to be set on content-visibility . This effectively enables content-visibility transitions. 

Note: 
When transitioning an element's content-visibility value, you don't need to provide a set of starting values for transitioned properties using a @starting-style block, like you do when transitioning display . This is because content-visibility doesn't hide an element from the DOM like display does: it just skips rendering the element's content. 

Formal definition 

Initial value visible 
Applies to elements for which size containment can apply 
Inherited no 
Computed value as specified 
Animation type Discrete behavior except when animating to or from hidden is visible for the entire duration 

Formal syntax 

content-visibility = 
visible | 
auto | 
hidden 

Accessibility 

Off-screen content within a content-visibility: auto property remains in the document object model and the accessibility tree. This allows improving page performance with content-visibility: auto without negatively impacting accessibility. 

Since styles for off-screen content are not rendered, elements intentionally hidden with display: none or visibility: hidden will still appear in the accessibility tree .
If you don't want an element to appear in the accessibility tree, use aria-hidden="true" . 

Examples 

Using auto to reduce rendering cost of long pages 

The following example shows the use of content-visibility: auto to skip painting and rendering of off-screen sections.
When a section is out of the viewport then the painting of the content is skipped until the section comes close to the viewport, this helps with both load and interactions on the page. 

HTML 

html 
<section>
<!-- Content for each section… -->
</section>
<section>
<!-- Content for each section… -->
</section>
<section>
<!-- Content for each section… -->
</section>
<!-- … -->

CSS 

The contain-intrinsic-size property adds a default size of 500px to the height and width of each section element. After a section is rendered, it will retain its rendered intrinsic size, even when it is scrolled out of the viewport. 

css 
section {
content-visibility: auto;
contain-intrinsic-size: auto 500px;
}

Using hidden to manage visibility 

The following example shows how to manage content visibility with JavaScript.
Using content-visibility: hidden; instead of display: none; preserves the rendering state of content when hidden and rendering is faster. 

HTML 

html 
<div class="hidden">
<button class="toggle">Show</button>
<p>
This content is initially hidden and can be shown by clicking the button.
</p>
</div>
<div class="visible">
<button class="toggle">Hide</button>
<p>
This content is initially visible and can be hidden by clicking the button.
</p>
</div>

CSS 

The content-visibility property is set on paragraphs that are direct children of elements with the visible and hidden classes. In our example, we can show and hide content in paragraphs depending on the CSS class of parent div elements. 

The contain-intrinsic-size property is included to represent the content size. This helps to reduce layout shift when content is hidden. 

css 
p {
contain-intrinsic-size: 0 1.1em;
border: dotted 2px;
}

.hidden > p {
content-visibility: hidden;
}

.visible > p {
content-visibility: visible;
}

JavaScript 

js 
const handleClick = (event) => {
const button = event.target;
const div = button.parentElement;
button.textContent = div.classList.contains("visible") ? "Show" : "Hide";
div.classList.toggle("hidden");
div.classList.toggle("visible");
};

document.querySelectorAll("button.toggle").forEach((button) => {
button.addEventListener("click", handleClick);
});

Result 

Animating content-visibility 

In this example, we have a <div> element, the content of which can be toggled between shown and hidden by clicking or pressing any key. 

HTML 

html 
<p>
Click anywhere on the screen or press any key to toggle the
<code><div></code> content between hidden and showing.
</p>

<div>
This is a <code><div></code> element that animates between
<code>content-visibility: hidden;</code>and
<code>content-visibility: visible;</code>. We've also animated the text color
to create a smooth animation effect.
</div>

CSS 

In the CSS we initially set content-visibility: hidden; on the <div> to hide its content. We then set up @keyframes animations and attach them to classes to show and hide the <div> , animating content-visibility and color so that you get a smooth animation effect as the content is shown/hidden. 

css 
div {
font-size: 1.6rem;
padding: 20px;
border: 3px solid red;
border-radius: 20px;
width: 480px;

content-visibility: hidden;
}

/* Animation classes */

.show {
animation: show 0.7s ease-in forwards;
}

.hide {
animation: hide 0.7s ease-out forwards;
}

/* Animation keyframes */

@keyframes show {
0% {
content-visibility: hidden;
color: transparent;
}

100% {
content-visibility: visible;
color: black;
}
}

@keyframes hide {
0% {
content-visibility: visible;
color: black;
}

100% {
content-visibility: hidden;
color: transparent;
}
}

JavaScript 

Finally, we use JavaScript to apply the .show and .hide classes to the <div> as appropriate to apply the animations as it is toggled between shown and hidden states. 

js 
const divElem = document.querySelector("div");
const htmlElem = document.querySelector(":root");

htmlElem.addEventListener("click", showHide);
document.addEventListener("keydown", showHide);

function showHide() {
if (divElem.classList.contains("show")) {
divElem.classList.remove("show");
divElem.classList.add("hide");
} else {
divElem.classList.remove("hide");
divElem.classList.add("show");
}
}

Result 

The rendered result looks like this: 

Specifications 

Specification 

CSS Containment Module Level 2 
# content-visibility 

Browser compatibility 

See also 

CSS Containment 

contain-intrinsic-size 

contentvisibilityautostatechange 

content-visibility: the new CSS property that boosts your rendering performance (web.dev) 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Dec 16, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# contain - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/contain

contain 

Baseline

Widely available

This feature is well established and works across many devices and browser versions. It’s been available across browsers since March 2022.

Learn more

See full compatibility

Report feedback

The contain CSS property indicates that an element and its contents are, as much as possible, independent from the rest of the document tree.
Containment enables isolating a subsection of the DOM, providing performance benefits by limiting calculations of layout, style, paint, size, or any combination to a DOM subtree rather than the entire page. Containment can also be used to scope CSS counters and quotes. 

Try it 

contain: none;

contain: size;

contain: layout;

contain: paint;

contain: strict;

<section class="default-example" id="default-example">
<div class="card" id="example-element">
<h2>Element with '<code>contain</code>'</h2>
<p>
The Goldfish is a species of domestic fish best known for its bright
colors and patterns.
</p>
<div class="fixed"><p>Fixed right 4px</p></div>
</div>
</section>

h2 {
margin-top: 0;
}

#default-example {
text-align: left;
padding: 4px;
font-size: 16px;
}

.card {
text-align: left;
border: 3px dotted;
padding: 20px;
margin: 10px;
width: 85%;
min-height: 150px;
}

.fixed {
position: fixed;
border: 3px dotted;
right: 4px;
padding: 4px;
margin: 4px;
}

Syntax 

css 
/* Keyword values */
contain: none;
contain: strict;
contain: content;
contain: size;
contain: inline-size;
contain: layout;
contain: style;
contain: paint;

/* Multiple keywords */
contain: size paint;
contain: size layout paint;
contain: inline-size layout;

/* Global values */
contain: inherit;
contain: initial;
contain: revert;
contain: revert-layer;
contain: unset;

Values 

The contain property can have any of the following values: 

The keyword none or 

One or more of the space-separated keywords size (or inline-size ), layout , style , and paint in any order or 

One of the shorthand values strict or content 

The keywords have the following meanings: 

none 

The element renders as normal, with no containment applied. 

strict 

All containment rules are applied to the element. This is equivalent to contain: size layout paint style . 

content 

All containment rules except size are applied to the element. This is equivalent to contain: layout paint style . 

size 

Size containment is applied to the element in both the inline and block directions. The size of the element can be computed in isolation, ignoring the child elements. This value cannot be combined with inline-size . 

inline-size 

Inline size containment is applied to the element. The inline size of the element can be computed in isolation, ignoring the child elements. This value cannot be combined with size . 

layout 

The internal layout of the element is isolated from the rest of the page. This means nothing outside the element affects its internal layout, and vice versa. 

style 

For properties that can affect more than just an element and its descendants, the effects don't escape the containing element. Counters and quotes are scoped to the element and its contents. 

paint 

Descendants of the element don't display outside its bounds. If the containing box is offscreen, the browser does not need to paint its contained elements — these must also be offscreen as they are contained completely by that box. If a descendant overflows the containing element's bounds, then that descendant will be clipped to the containing element's border-box. 

Description 

There are four types of CSS containment: size, layout, style, and paint, which are set on the container.
The property is a space-separated list of a subset of the five standard values or one of the two shorthand values.
Changes to the contained properties within the container are not propagated outside of the contained element to the rest of the page.
The main benefit of containment is that the browser does not have to re-render the DOM or page layout as often, leading to small performance benefits during the rendering of static pages and greater performance benefits in more dynamic applications. 

Using the contain property is useful on pages with groups of elements that are supposed to be independent, as it can prevent element internals from having side effects outside of its bounding-box. 

Note: 
Using layout , paint , strict or content values for this property creates: 

A new containing block (for the descendants whose position property is absolute or fixed ). 

A new stacking context . 

A new block formatting context . 

Formal definition 

Initial value none 
Applies to all elements 
Inherited no 
Computed value as specified 
Animation type Not animatable 

Formal syntax 

contain = 
none | 
strict | 
content | 
[ [ size | inline-size ] || layout || style || paint ] 

Examples 

Paint containment 

The following example shows how to use contain: paint to prevent an element's descendants from painting outside of its bounds. 

css 
div {
width: 100px;
height: 100px;
background: red;
margin: 10px;
font-size: 20px;
}

.contain-paint {
contain: paint;
}

html 
<div class="contain-paint">
<p>This text will be clipped to the bounds of the box.</p>
</div>
<div>
<p>This text will not be clipped to the bounds of the box.</p>
</div>

Layout containment 

Consider the example below which shows how elements behave with and without layout containment applied: 

html 
<div class="card contain-layout">
<h2>Card 1</h2>
<div class="fixed"><p>Fixed box 1</p></div>
<div class="float"><p>Float box 1</p></div>
</div>
<div class="card">
<h2>Card 2</h2>
<div class="fixed"><p>Fixed box 2</p></div>
<div class="float"><p>Float box 2</p></div>
</div>
<div class="card">
<h2>Card 3</h2>
<!-- ... -->
</div>

p {
margin: 4px;
padding: 4px;
}

h2 {
margin-bottom: 4px;
padding: 10px;
}

div {
border-radius: 4px;
box-shadow: 0 2px 4px 0 gray;
padding: 6px;
margin: 6px;
}

css 
.card {
width: 70%;
height: 90px;
}

.fixed {
position: fixed;
right: 10px;
top: 10px;
background: coral;
}

.float {
float: left;
margin: 10px;
background: aquamarine;
}

.contain-layout {
contain: layout;
}

The first card has layout containment applied, and its layout is isolated from the rest of the page.
We can reuse this card in other places on the page without worrying about layout recalculation of the other elements.
If floats overlap the card bounds, elements on the rest of the page are not affected.
When the browser recalculates the containing element's subtree, only that element is recalculated. Nothing outside of the contained element needs to be recalculated.
Additionally, the fixed box uses the card as a layout container to position itself. 

The second and third cards have no containment.
The layout context for the fixed box in the second card is the root element so the fixed box is positioned in the top right corner of the page.
A float overlaps the second card's bounds causing the third card to have unexpected layout shift that's visible in the positioning of the <h2> element.
When recalculation occurs, it is not limited to a container.
This impacts performance and interferes with the rest of the page layout. 

Style containment 

Style containment scopes counters and quotes to the contained element.
For CSS counters, the counter-increment and counter-set properties are scoped to the element as if the element is at the root of the document. 

Containment and counters 

The example below takes a look at how counters work when style containment is applied: 

html 
<ul>
<li>Item A</li>
<li>Item B</li>
<li class="container">Item C</li>
<li>Item D</li>
<li>Item E</li>
</ul>

css 
body {
counter-reset: list-items;
}

li::before {
counter-increment: list-items;
content: counter(list-items) ": ";
}

.container {
contain: style;
}

Without containment, the counter would increment from 1 to 5 for each list item.
Style containment causes the counter-increment property to be scoped to the element's subtree and the counter begins again at 1: 

Containment and quotes 

CSS quotes are similarly affected in that the content values relating to quotes are scoped to the element: 

html 
<!-- With style containment -->
<span class="open-quote">
outer
<span class="contain-style">
<span class="open-quote">inner</span>
</span>
</span>
<span class="close-quote">close</span>
<br />
<!-- Without containment -->
<span class="open-quote">
outer
<span>
<span class="open-quote">inner</span>
</span>
</span>
<span class="close-quote">close</span>

css 
body {
quotes: "[" "]" "‹" "›";
}
.open-quote::before {
content: open-quote;
}

.close-quote::after {
content: close-quote;
}

.contain-style {
contain: style;
}

Because of containment, the first closing quote ignores the inner span and uses the outer span's closing quote instead: 

Specifications 

Specification 

CSS Containment Module Level 2 
# contain-property 

Browser compatibility 

See also 

CSS containment 

CSS container queries 

CSS content-visibility property 

CSS position property 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Dec 5, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# will-change - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/will-change

will-change 

Baseline

Widely available

This feature is well established and works across many devices and browser versions. It’s been available across browsers since January 2020.

Learn more

See full compatibility

Report feedback

The will-change CSS property hints to browsers how an element is expected to change. Browsers may set up optimizations before an element is actually changed. These kinds of optimizations can increase the responsiveness of a page by doing potentially expensive work before they are actually required. 

Warning: 
will-change is intended to be used as a last resort, in order to try to deal with existing performance problems. It should not be used to anticipate performance problems. 

Proper usage of this property can be a bit tricky: 

Don't apply will-change to too many elements. The browser already tries as hard as it can to optimize everything. Some of the stronger optimizations that are likely to be tied to will-change end up using a lot of a machine's resources, and when overused like this can cause the page to slow down or consume a lot of resources. 

Use sparingly. The normal behavior for optimizations that the browser make is to remove the optimizations as soon as it can and revert back to normal. But adding will-change directly in a stylesheet implies that the targeted elements are always a few moments away from changing and the browser will keep the optimizations for much longer time than it would have otherwise. So it is a good practice to switch will-change on and off using script code before and after the change occurs. 

Don't apply will-change to elements to perform premature optimization . If your page is performing well, don't add the will-change property to elements just to wring out a little more speed. will-change is intended to be used as something of a last resort, in order to try to deal with existing performance problems. It should not be used to anticipate performance problems. Excessive use of will-change will result in excessive memory use and will cause more complex rendering to occur as the browser attempts to prepare for the possible change. This will lead to worse performance. 

Give it sufficient time to work . This property is intended as a method for authors to let the user-agent know about properties that are likely to change ahead of time. Then the browser can choose to apply any ahead-of-time optimizations required for the property change before the property change actually happens. So it is important to give the browser some time to actually do the optimizations. Find some way to predict at least slightly ahead of time that something will change, and set will-change then. 

Be aware, that will-change may actually influence the visual appearance of elements , when used with property values, that create a stacking context (e.g., will-change: opacity), as the stacking context is created up front. 

Syntax 

css 
/* Keyword values */
will-change: auto;
will-change: scroll-position;
will-change: contents;
will-change: transform; /* Example of <custom-ident> */
will-change: opacity; /* Example of <custom-ident> */
will-change: left, top; /* Example of two <animatable-feature> */

/* Global values */
will-change: inherit;
will-change: initial;
will-change: revert;
will-change: revert-layer;
will-change: unset;

Values 

auto 

This keyword expresses no particular intent; the user agent should apply whatever heuristics and optimizations it normally does. 

The <animatable-feature> can be one of the following values: 

scroll-position 

Indicates that the author expects to animate or change the scroll position of the element in the near future. 

contents 

Indicates that the author expects to animate or change something about the element's contents in the near future. 

<custom-ident> 

Indicates that the author expects to animate or change the property with the given name on the element in the near future. If the property given is a shorthand, it indicates the expectation for all the longhands the shorthand expands to. It cannot be one of the following values: unset , initial , inherit , will-change , auto , scroll-position , or contents . The spec doesn't define the behavior of particular value, but it is common for transform to be a compositing layer hint. Chrome currently takes two actions , given particular CSS property idents: establish a new compositing layer or a new stacking context . 

Via stylesheet 

It may be appropriate to include will-change in your style sheet for an application that does page flips on key presses like an album or a slide deck presentation where the pages are large and complex. This will let browser prepare the transition ahead of time and allow for snappy transitions between the pages as soon as the key is pressed. But use caution with the will-change property directly in stylesheets. It may cause the browser to keep the optimization in memory for much longer than it is needed. 

css 
.slide {
will-change: transform;
}

Formal definition 

Initial value auto 
Applies to all elements 
Inherited no 
Computed value as specified 
Animation type discrete 

Formal syntax 

will-change = 
auto | 
<animateable-feature> # 

"><animateable-feature> = 
scroll-position | 
contents | 
<custom-ident> 

Examples 

Via script 

This is an example showing how to apply the will-change property through scripting, which is probably what you should be doing in most cases. 

js 
const el = document.getElementById("element");

// Set will-change when the element is hovered
el.addEventListener("mouseenter", hintBrowser);
el.addEventListener("animationEnd", removeHint);

function hintBrowser() {
// The optimizable properties that are going to change
// in the animation's keyframes block
this.style.willChange = "transform, opacity";
}

function removeHint() {
this.style.willChange = "auto";
}

Specifications 

Specification 

CSS Will Change Module Level 1 
# will-change 

Browser compatibility 

See also 

transform 

Individual transform properties:

translate 

scale 

rotate 

Note: there is no individual skew property 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Nov 7, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content