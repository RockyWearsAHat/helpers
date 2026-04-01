# grid-template-columns - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-columns#subgrid

grid-template-columns 

Baseline

Widely available

*

This feature is well established and works across many devices and browser versions. It’s been available across browsers since October 2017.

* Some parts of this feature may have varying levels of support. 

Learn more

See full compatibility

Report feedback

The grid-template-columns CSS property defines the line names and track sizing functions of the grid columns . 

Try it 

grid-template-columns: 60px 60px;

grid-template-columns: 1fr 60px;

grid-template-columns: 1fr 2fr;

grid-template-columns: 8ch auto;

<section class="default-example" id="default-example">
<div class="example-container">
<div class="transition-all" id="example-element">
<div>One</div>
<div>Two</div>
<div>Three</div>
<div>Four</div>
<div>Five</div>
</div>
</div>
</section>

#example-element {
border: 1px solid #c5c5c5;
display: grid;
grid-auto-rows: 40px;
grid-gap: 10px;
width: 200px;
}

#example-element > div {
background-color: rgb(0 0 255 / 0.2);
border: 3px solid blue;
}

Syntax 

css 
/* Keyword value */
grid-template-columns: none;

/* <track-list> values */
grid-template-columns: 100px 1fr;
grid-template-columns: [line-name] 100px;
grid-template-columns: [line-name1] 100px [line-name2 line-name3];
grid-template-columns: minmax(100px, 1fr);
grid-template-columns: fit-content(40%);
grid-template-columns: repeat(3, 200px);
grid-template-columns: subgrid;
grid-template-columns: masonry;

/* <auto-track-list> values */
grid-template-columns: 200px repeat(auto-fill, 100px) 300px;
grid-template-columns:
minmax(100px, max-content)
repeat(auto-fill, 200px) 20%;
grid-template-columns:
[line-name1] 100px [line-name2]
repeat(auto-fit, [line-name3 line-name4] 300px)
100px;
grid-template-columns:
[line-name1 line-name2] 100px
repeat(auto-fit, [line-name1] 300px) [line-name3];

/* Global values */
grid-template-columns: inherit;
grid-template-columns: initial;
grid-template-columns: revert;
grid-template-columns: revert-layer;
grid-template-columns: unset;

Values 

none 

Indicates that there is no explicit grid. Any columns will be implicitly generated and their size will be determined by the grid-auto-columns property. 

[line-name] 

A <custom-ident> specifying a name for the line in that location. The ident may be any valid string other than the reserved words span and auto . Lines may have multiple names separated by a space inside the square brackets, for example [line-name-a line-name-b] . 

<length> 

A non-negative length, giving the width of the column. 

<percentage> 

A non-negative <percentage> value relative to the inline size of the grid container. If the size of the grid container depends on the size of its tracks, the browser treats the percentage as auto .
The browser may adjust the intrinsic size contributions of the track to the size of the grid container and may increase the final size of the track by the minimum amount that would result in honoring the percentage. 

<flex> 

Is a non-negative dimension with the unit fr specifying the track's flex factor. Each <flex> -sized track takes a share of the remaining space in proportion to its flex factor. 

When appearing outside a minmax() notation, it implies an automatic minimum (i.e., minmax(auto, <flex>) ). 

max-content 

Is a keyword representing the largest maximal content contribution of the grid items occupying the grid track. For example, if the first element of the grid track contains the sentence "Repetitio est mater studiorum" and the second element contains the sentence "Dum spiro, spero" , maximal content contribution will be defined by the size of the largest sentence among all of the grid elements - "Repetitio est mater studiorum" . 

min-content 

Is a keyword representing the largest minimal content contribution of the grid items occupying the grid track. For example, if the first element of the grid track contains the sentence "Repetitio est mater studiorum" and the second element contains the sentence "Dum spiro, spero" , minimal content contribution will be defined by the size of the largest word among all of the sentences in the grid elements - "studiorum" . 

minmax(min, max) 

Is a functional notation that defines a size range greater than or equal to min and less than or equal to max . If max is smaller than min , then max is ignored and the function is treated as min . As a maximum, a <flex> value sets the track's flex factor. It is invalid as a minimum. 

auto 

As a maximum value, it represents the largest max-content size of the items in that track. 

As a minimum value, it represents the largest minimum size of items in that track (specified by the min-width / min-height properties of the items). This often corresponds to the min-content size, but not always. 

If used outside of minmax() notation, auto represents the range between the minimum and maximum values described above. In most cases, this behaves similarly to minmax(min-content,max-content) . 

Note: 
auto track sizes (and only auto track sizes) can be stretched by the align-content and justify-content properties. Therefore, by default, an auto -sized track will take up any remaining space in the grid container. 

fit-content( [ <length> | <percentage> ] ) 

Represents the formula max(minimum, min(limit, max-content)) , where minimum represents an auto minimum (which is often, but not always, equal to a min-content minimum), and limit is the track sizing function passed as an argument to fit-content(). This is essentially calculated as the smaller of minmax(auto, max-content) and minmax(auto, limit) . 

repeat( [ <positive-integer> | auto-fill | auto-fit ] , <track-list> ) 

Represents a repeated fragment of the track list, allowing a large number of columns that exhibit a recurring pattern to be written in a more compact form. 

masonry 

The masonry value indicates that this axis should be laid out according to the masonry algorithm. 

subgrid 

The subgrid value indicates that the grid will adopt the spanned portion of its parent grid in that axis. Rather than being specified explicitly, the sizes of the grid rows/columns will be taken from the parent grid's definition. 

Formal definition 

Initial value none 
Applies to grid containers 
Inherited no 
Percentages refer to corresponding dimension of the content area 
Computed value as specified, but with relative lengths converted into absolute lengths 
Animation type simple list of length, percentage, or calc, provided the only differences are in the values of the length, percentage, or calc components in the list 

Formal syntax 

grid-template-columns = 
none | 
<track-list> | 
<auto-track-list> | 
subgrid <line-name-list> ? 

"><track-list> = 
[ <line-names> ? [ <track-size> | <track-repeat> ] ] + <line-names> ? 

"><auto-track-list> = 
[ <line-names> ? [ <fixed-size> | <fixed-repeat> ] ] * <line-names> ? <auto-repeat> [ <line-names> ? [ <fixed-size> | <fixed-repeat> ] ] * <line-names> ? 

"><line-name-list> = 
[ <line-names> | <name-repeat> ] + 

"><line-names> = 
'[' <custom-ident> * ']' 

"><track-size> = 
<track-breadth> | 
minmax( <inflexible-breadth> , <track-breadth> ) | 
fit-content( <length-percentage [0,∞]> ) 

"><track-repeat> = 
repeat( [ <integer [1,∞]> ] , [ <line-names> ? <track-size> ] + <line-names> ? ) 

"><fixed-size> = 
<fixed-breadth> | 
minmax( <fixed-breadth> , <track-breadth> ) | 
minmax( <inflexible-breadth> , <fixed-breadth> ) 

"><fixed-repeat> = 
repeat( [ <integer [1,∞]> ] , [ <line-names> ? <fixed-size> ] + <line-names> ? ) 

"><auto-repeat> = 
repeat( [ auto-fill | auto-fit ] , [ <line-names> ? <track-size> ] + <line-names> ? ) 

"><name-repeat> = 
repeat( [ <integer [1,∞]> | auto-fill ] , <line-names> + ) 

"><track-breadth> = 
<length-percentage [0,∞]> | 
<flex [0,∞]> | 
min-content | 
max-content | 
auto 

"><inflexible-breadth> = 
<length-percentage [0,∞]> | 
min-content | 
max-content | 
auto 

"><length-percentage> = 
<length> | 
<percentage> 

"><integer> = 
<number-token> 

"><fixed-breadth> = 
<length-percentage [0,∞]> 

Examples 

Specifying grid column sizes 

HTML 

html 
<div id="grid">
<div id="areaA">A</div>
<div id="areaB">B</div>
</div>

CSS 

css 
#grid {
display: grid;
width: 100%;
grid-template-columns: 50px 1fr;
}

#areaA {
background-color: lime;
}

#areaB {
background-color: yellow;
}

Result 

Specifications 

Specification 

CSS Grid Layout Module Level 2 
# track-sizing 

CSS Grid Layout Module Level 2 
# subgrids 

Browser compatibility 

See also 

grid-template-rows 

grid-template-areas 

grid-template 

Basic concepts of grid layout: grid tracks 

Video: Defining a grid 

Subgrid 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Dec 16, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# CSS nesting - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Nesting

CSS nesting 

The CSS nesting module defines a syntax for nesting selectors, providing the ability to nest one style rule inside another, with the selector of the child rule relative to the selector of the parent rule. 

CSS nesting is different from CSS preprocessors such as Sass in that it is parsed by the browser rather than being pre-compiled by a CSS preprocessor. 

CSS nesting helps with the readability, modularity, and maintainability of CSS stylesheets. It also potentially helps reduce the size of CSS files, thereby decreasing the amount of data downloaded by users. 

Reference 

Selectors 

& nesting selector 

Guides 

Using CSS nesting 

Explains how to use CSS nesting. 

CSS nesting at-rules 

Explains how to nest at-rules. 

CSS nesting and specificity 

Explains the differences in specificity when nesting CSS. 

Related concepts 

Selectors and combinators 

Pseudo-classes 

CSS preprocessor 

Specifications 

Specification 

CSS Nesting Module Level 1 

See also 

Specificity 

CSS cascading and inheritance module 

CSS selectors module 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Nov 18, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# :has() - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/:has

:has() 

Baseline

2023

Newly available

Since December 2023, this feature works across the latest devices and browser versions. This feature might not work in older devices or browsers.

Learn more

See full compatibility

Report feedback

The functional :has() CSS pseudo-class represents an element if any of the relative selectors that are passed as an argument match at least one element when anchored against this element. This pseudo-class presents a way of selecting a parent element or a previous sibling element with respect to a reference element by taking a relative selector list as an argument. 

css 
/* Selects an h1 heading with a
paragraph element that immediately follows
the h1 and applies the style to h1 */
h1:has(+ p) {
margin-bottom: 0;
}

The :has() pseudo-class takes on the specificity of the most specific selector in its arguments the same way as :is() and :not() do. 

Syntax 

css 
:has(<relative-selector-list>) {
/* ... */
}

If the :has() pseudo-class itself is not supported in a browser, the entire selector block will fail unless :has() is in a forgiving selector list, such as in :is() and :where() . 

The :has() pseudo-class cannot be nested within another :has() . 

Pseudo-elements are also not valid selectors within :has() and pseudo-elements are not valid anchors for :has() . This is because many pseudo-elements exist conditionally based on the styling of their ancestors and allowing these to be queried by :has() can introduce cyclic querying. 

Examples 

Selecting a parent element 

You may be looking for a "parent combinator ", which allows you to go up the DOM tree and select the parent of a specific element. The :has() pseudo-class does that by using parent:has(child) (for any parent) or parent:has(> child) (for direct parent). This example shows how to style a <section> element when it contains a child with the featured class. 

html 
<section>
<article class="featured">Featured content</article>
<article>Regular content</article>
</section>
<section>
<article>Regular content</article>
</section>

css 
section:has(.featured) {
border: 2px solid blue;
}

Result 

With the sibling combinator 

The :has() style declaration in the following example adjusts the spacing after <h1> headings if they are immediately followed by an <h2> heading. 

HTML 

html 
<section>
<article>
<h1>Morning Times</h1>
<p>
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</p>
</article>
<article>
<h1>Morning Times</h1>
<h2>Delivering you news every morning</h2>
<p>
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</p>
</article>
</section>

CSS 

section {
display: flex;
align-items: start;
justify-content: space-around;
}

article {
display: inline-block;
width: 40%;
}

h1,
h2 {
font-size: 1.2em;
}

h2 {
font-size: 1em;
color: rgb(150 149 149);
}

css 
h1,
h2 {
margin: 0 0 1rem 0;
}

h1:has(+ h2) {
margin: 0 0 0.25rem 0;
}

Result 

This example shows two similar texts side-by-side for comparison – the left one with an H1 heading followed by a paragraph and the right one with an H1 heading followed by an H2 heading and then a paragraph. In the example on the right, :has() helps to select the H1 element that is immediately followed by an H2 element (indicated by the next-sibling combinator + ) and the CSS rule reduces the spacing after such an H1 element. Without the :has() pseudo-class, you cannot use CSS selectors to select a preceding sibling of a different type or a parent element. 

With the :is() pseudo-class 

This example builds on the previous example to show how to select multiple elements with :has() . 

HTML 

html 
<section>
<article>
<h1>Morning Times</h1>
<h2>Delivering you news every morning</h2>
<p>
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</p>
</article>
<article>
<h1>Morning Times</h1>
<h2>Delivering you news every morning</h2>
<h3>8:00 am</h3>
<p>
Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua.
</p>
</article>
</section>

CSS 

section {
display: flex;
align-items: start;
justify-content: space-around;
}

article {
display: inline-block;
width: 40%;
}

h1 {
font-size: 1.2em;
}

h2 {
font-size: 1em;
color: rgb(150 149 149);
}

h3 {
font-size: 0.9em;
color: darkgrey;
}

css 
h1,
h2,
h3 {
margin: 0 0 1rem 0;
}

:is(h1, h2, h3):has(+ :is(h2, h3, h4)) {
margin: 0 0 0.25rem 0;
}

Result 

Here, the first :is() pseudo-class is used to select any of the heading elements in the list. The second :is() pseudo-class is used to pass a list of next-sibling selectors as an argument to :has() . The :has() pseudo-class helps to select any H1 , H2 , or H3 element that is immediately followed by (indicated by + ) an H2 , H3 , or H4 element and the CSS rule reduces the spacing after such H1 , H2 , or H3 elements. 

This selector could have also been written as: 

css 
:is(h1, h2, h3):has(+ h2, + h3, + h4) {
margin: 0 0 0.25rem 0;
}

Logical operations 

The :has() relational selector can be used to check if one of the multiple features is true or if all the features are true. 

By using comma-separated values inside the :has() relational selector, you are checking to see if any of the parameters exist. x:has(a, b) will style x if descendant a OR b exists. 

By chaining together multiple :has() relational selectors together, you are checking to see if all of the parameters exist. x:has(a):has(b) will style x if descendant a AND b exist. 

css 
body:has(video, audio) {
/* styles to apply if the content contains audio OR video */
}
body:has(video):has(audio) {
/* styles to apply if the content contains both audio AND video */
}

Analogy between :has() and regular expressions 

Interestingly, we can relate some CSS :has() constructs with the lookahead assertion in regular expressions because they both allow you to select elements (or strings in regular expressions) based on a condition without actually selecting the condition matching the element (or string) itself. 

Positive lookahead (?=pattern) 

In the regular expression abc(?=xyz) , the string abc is matched only if it is immediately followed by the string xyz . As it is a lookahead operation, the xyz is not included in the match. 

The analogous construct in CSS would be .abc:has(+ .xyz) : it selects the element .abc only if there is a next sibling .xyz . The part :has(+ .xyz) acts as a lookahead operation because the element .abc is selected and not the element .xyz . 

Negative lookahead (?!pattern) 

Similarly, for the negative lookahead case, in the regular expression abc(?!xyz) , the string abc is matched only if it is not followed by xyz . The analogous CSS construct .abc:has(+ :not(.xyz)) doesn't select the element .abc if the next element is .xyz . 

Performance considerations 

Certain uses of the :has() pseudo-class can significantly impact page performance, particularly during dynamic updates (DOM mutations). Browser engines must re-evaluate :has() selectors when the DOM changes, and complex or poorly constrained selectors can lead to expensive computations. 

Avoid broad anchoring 

The anchor selector (the A in A:has(B) ) should not be an element that has too many children, like body , :root , or * . Anchoring :has() to very general selectors can degrade performance because any DOM change within the entire subtree of a broadly selected element requires the browser to re-check the :has() condition. 

css 
/* Avoid anchoring :has() to broad elements */
body:has(.sidebar) {
/* styles */
}
:root:has(.content) {
/* styles */
}
*:has(.item) {
/* styles */
}

Instead, anchor :has() to specific elements like .container or .gallery to reduce the scope and improve performance. 

css 
/* Use specific containers to limit scope */
.container:has(.sidebar-expanded) {
/* styles */
}
.content-wrapper:has(> article[data-priority="high"]) {
/* styles */
}
.gallery:has(> img[data-loaded="false"]) {
/* styles */
}

Minimize subtree traversals 

The inner selector (the B in A:has(B) ) should use combinators like > or + to limit traversal. When the selector inside :has() is not tightly constrained, the browser might need to traverse the entire subtree of the anchor element on every DOM mutation to check if the condition still holds. 

In this example, any change within .ancestor requires checking all descendants for .foo : 

css 
/* May trigger full subtree traversal */
.ancestor:has(.foo) {
/* styles */
}

Using child or sibling combinators limits the scope of the inner selector, reducing the performance cost of DOM mutations. In this example, the browser only needs to check direct children or a specific sibling's descendants: 

css 
/* More constrained - limits traversal */
.ancestor:has(> .foo) {
/* direct child */
}
.ancestor:has(+ .sibling .foo) {
/* descendant of adjacent sibling */
}

Certain inner selectors can force the browser to traverse up the ancestor chain for every DOM mutation, looking for potential anchors that might need updating. This happens when the structure implies a need to check ancestors of the mutated element. 

In this example, any DOM change requires checking if the changed element is any element ( * ) that is a direct child of .foo , and if its parent (or further ancestors) is .ancestor . 

css 
/* Might trigger ancestor traversal */
.ancestor:has(.foo > *) {
/* styles */
}

Constraining the inner selector with specific classes or direct child combinators (e.g., .specific-child in the next snippet) reduces expensive ancestor traversals by limiting the browser's check to a well-defined element, improving performance. 

css 
/* Constrain the inner selector to avoid ancestor traversals */
.ancestor:has(.foo > .specific-child) {
/* styles */
}

Note: 
These performance characteristics may improve as browsers optimize :has() implementations, but the fundamental constraints remain: :has() needs to traverse a whole subtree, so you need to minimize the subtree's size. In a selector like A:has(B) , make sure your A does not have too many children, and make sure your B is tightly constrained to avoid unnecessary traversal. 

Specifications 

Specification 

Selectors Level 4 
# relational 

Browser compatibility 

See also 

:is() , :where() , :not() 

CSS selectors and combinators 

CSS selector structure 

Selector list 

CSS selector module 

Selection and traversal on the DOM tree 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Dec 16, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content