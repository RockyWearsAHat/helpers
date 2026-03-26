# @layer - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@layer

@layer 

Baseline

Widely available

This feature is well established and works across many devices and browser versions. It’s been available across browsers since March 2022.

Learn more

See full compatibility

Report feedback

The @layer CSS at-rule is used to declare a cascade layer and can also be used to define the order of precedence in case of multiple cascade layers. 

Try it 

@layer module, state;

@layer state {
.alert {
background-color: brown;
}
p {
border: medium solid limegreen;
}
}

@layer module {
.alert {
border: medium solid violet;
background-color: yellow;
color: white;
}
}

<p class="alert">Beware of the zombies</p>

Syntax 

css 
/* statement at-rules */
@layer layer-name;
@layer layer-name, layer-name, layer-name;

/* block at-rules */
@layer {rules}
@layer layer-name {rules}

where: 

layer-name 

Is the name of the cascade layer. 

rules 

Is the set of CSS rules in the cascade layer. 

Description 

Rules within a cascade layer cascade together, giving more control over the cascade to web developers. Styles that are not defined in a layer always override styles declared in named and anonymous layers. 

The following diagram shows layer priorities where layers are declared in 1, 2, ..., N order. 

As noted in the above diagram, important declarations , declarations with the !important flag, have priority over normal declarations , or regular declarations without the !important flag. The order of precedence among important rules is the inverse of normal rules. Transitions have the greatest precedence. Next in order of highest to lowest priority are the important user agent declarations, important user declarations, and important author declarations; in that order. Users can specify styles using browser preferences, operating system preferences, or browser extensions. Their important declarations take precedence over author , or web developer written, important declarations. 

Within author styles, all important declarations within CSS layers take precedence over any important declarations declared outside of a layer, while all normal declarations within CSS layers have a lower priority than declarations declared outside of a layer.
The declaration order matters. The first declared layer gets the lowest priority and the last declared layer gets the highest priority. However, the priority is reversed when the !important flag is used. 

The @layer at-rule is used to create a cascade layer in one of three ways. 

The first way is to use a @layer block at-rule to create a named cascade layer with the CSS rules for that layer inside, like so: 

css 
@layer utilities {
.padding-sm {
padding: 0.5rem;
}

.padding-lg {
padding: 0.8rem;
}
}

The second way is to use a @layer statement at-rule to create one or more comma-separated named cascade layers without assigning any styles. This can be a single layer, as shown below: 

css 
@layer utilities;

Multiple layers can be defined at once, as shown below: 

css 
@layer theme, layout, utilities;

This is useful because the initial order in which layers are declared indicates which layer has precedence. As with declarations, the last layer to be listed will win if declarations are found in multiple layers. Therefore, with the preceding example, if a competing rule was found in theme and utilities , the one in utilities would win and be applied. 

A rule in utilities would be applied even if it has lower specificity than the rule in theme . This is because once the layer order has been established, specificity and order of appearance are ignored. This enables using simpler CSS selectors because you do not have to ensure that a selector will have high enough specificity to override competing rules; all you need to ensure is that it appears in a later layer. 

Note: 
Having declared your layer names, thus setting their order, you can add CSS rules to the layer by re-declaring the name. The styles are then appended to the layer and the layer order will not be changed. 

The third way is to create an unnamed layer using a @layer block at-rule without including a layer name. For example: 

css 
@layer {
p {
margin-block: 1rem;
}
}

This creates an anonymous cascade layer . This layer functions in the same way as named layers; however, rules cannot be assigned to it later. The order of precedence for anonymous layers is the order in which layers are declared, named or not, and lower than the styles declared outside of a layer. 

Another way to create a cascade layer is by using @import . In this case, the rules would be in the imported stylesheet. Remember that the @import at-rule must precede all other types of rules, except @charset and @layer statements (not @layer blocks). 

css 
@import "theme.css" layer(utilities);

Nesting layers 

Layers may be nested. For example: 

css 
@layer framework {
@layer layout {
}
}

To append rules to the layout layer inside framework , join the two names with a . . 

css 
@layer framework.layout {
p {
margin-block: 1rem;
}
}

Formal syntax 

@layer = 
@layer <layer-name> ? { <rule-list> } | 
@layer <layer-name> # ; 

"><layer-name> = 
<ident> [ '.' <ident> ] * 

Examples 

Basic example 

In the following example, two CSS rules are created. One for the <p> element outside of any layer and one inside a layer named type for .box p . 

Without layers, the selector .box p would have the highest specificity, and therefore, the text Hello, world! will display in green. As the type layer comes before the anonymous layer created to hold non-layer content, the text will be purple. 

Also notice the order. Even though we declare the non-layered style first, it's still applied after the layer styles. 

HTML 

html 
<div class="box">
<p>Hello, world!</p>
</div>

CSS 

css 
p {
color: rebeccapurple;
}

@layer type {
.box p {
font-weight: bold;
font-size: 1.3em;
color: green;
}
}

Result 

Assigning rules to existing layers 

In the following example, two layers are created with no rules applied, then CSS rules are applied to the two layers. The base layer defines a color , border , font-size , and padding . The special layer defines a different color. As special comes last when the layers were defined, the color it provides is used and the text is displayed using rebeccapurple . All of the other rules from base still apply. 

HTML 

html 
<div class="item">
I am displayed in <code>color: rebeccapurple</code> because the
<code>special</code> layer comes after the <code>base</code> layer. My green
border, font-size, and padding come from the <code>base</code> layer.
</div>

CSS 

css 
@layer base, special;

@layer special {
.item {
color: rebeccapurple;
}
}

@layer base {
.item {
color: green;
border: 5px solid green;
font-size: 1.3em;
padding: 0.5em;
}
}

Result 

Specifications 

Specification 

CSS Cascading and Inheritance Level 5 
# layering 

Browser compatibility 

See also 

@import 

CSSLayerBlockRule 

CSSLayerStatementRule 

!important 

revert-layer 

Introducing the CSS cascade 

Learn: Handling conflicts 

Learn: Cascade layers 

The future of CSS: Cascade layers on bram.us (2021) 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Jan 21, 2026 by MDN contributors .

View this page on GitHub • Report a problem with this content

---

# Introduction to the CSS cascade - CSS | MDN
Source: https://developer.mozilla.org/en-US/docs/Web/CSS/Cascade

Introduction to the CSS cascade 

The cascade is an algorithm that defines how user agents combine property values originating from different sources. The cascade defines the origin and layer that takes precedence when declarations in more than one origin , cascade layer , or @scope block set a value for a property on an element. 

The cascade lies at the core of CSS, as emphasized by the name: Cascading Style Sheets. When a selector matches an element, the property value from the origin with the highest precedence gets applied, even if the selector from a lower precedence origin or layer has greater specificity . 

This article explains what the cascade is and the order in which CSS declarations cascade, covering cascade layers and origin type. Understanding origin precedence is key to understanding the cascade. 

Origin types 

The CSS cascade algorithm's job is to select CSS declarations in order to determine the correct values for CSS properties. CSS declarations come from different origin types: User-agent stylesheets , Author stylesheets , and User stylesheets . 

Though stylesheets come from these different origins and can be within different layers in each of these origins, they overlap in terms of their default scope; to make this work, the cascade algorithm defines how they interact. Before addressing the interactions, we'll define some key terms in the next few sections. 

User-agent stylesheets 

User-agents, or browsers, have basic stylesheets that give default styles to any document. These stylesheets are named user-agent stylesheets . Most browsers use actual stylesheets for this purpose, while others simulate them in code. The end result is the same. 

Some browsers let users modify the user-agent stylesheet, but this is rare and not something that can be controlled. 

Although some constraints on user-agent stylesheets are set by the HTML specification, browsers have a lot of latitude: that means some differences exist between browsers. To simplify the development process, Web developers may use a CSS reset stylesheet, such as normalize.css , which sets common properties values to a known state for all browsers before beginning to make alterations to suit their specific needs. 

Unless the user-agent stylesheet includes an !important next to a property, making it "important", styles declared by author styles, including a reset stylesheet, take precedence over the user-agent styles, regardless of the specificity of the associated selector. 

Author stylesheets 

Author stylesheets are the most common type of stylesheet; these are the styles written by web developers. These styles can reset user-agent styles, as noted above, and define the styles for the design of a given web page or application. The author, or web developer, defines the styles for the document using one or more linked or imported stylesheets, <style> blocks, and inline styles defined with the style attribute. These author styles define the look and feel of the website — its theme. 

User stylesheets 

In most browsers, the user (or reader) of the website can choose to override styles using a custom user stylesheet designed to tailor the experience to the user's wishes. Depending on the user agent, user styles can be configured directly or added via browser extensions. 

Cascade layers 

The cascade order is based on origin type. The cascade within each origin type is based on the declaration order of cascade layers within that type. For all origins - user-agent, author, or user - styles can be declared within or outside of named or anonymous layers. When declared using layer , layer() or @layer , styles are placed into the specified named layer, or into an anonymous layer if no name is provided. Styles declared outside of a layer are treated as being part of an anonymous last declared layer. 

Let's take a look at cascading origin type before diving into cascade layers within each origin type. 

Cascading order 

The cascading algorithm determines how to find the value to apply for each property for each document element. The following steps apply to the cascading algorithm: 

Relevance : It first filters all the rules from the different sources to keep only the rules that apply to a given element. That means rules whose selector matches the given element and which are part of an appropriate media at-rule. 

Origin and importance : Then it sorts these rules according to their importance, that is, whether or not they are followed by !important , and by their origin. Ignoring layers for the moment, the cascade order is as follows: 

Precedence Order (low to high) 
Origin 
Importance 

1 
user-agent (browser) 
normal 

2 
user 
normal 

3 
author (developer) 
normal 

4 
CSS keyframe animations 

5 
author (developer) 
!important 

6 
user 
!important 

7 
user-agent (browser) 
!important 

8 
CSS transitions 

Specificity : In case of equality with an origin, the specificity of a rule is considered to choose one value or another. The specificity of the selectors are compared, and the declaration with the highest specificity wins. 

Scoping proximity : When two selectors in the origin layer with precedence have the same specificity, the property value within scoped rules with the smallest number of hops up the DOM hierarchy to the scope root wins. See How @scope conflicts are resolved for more details and an example. 

Order of appearance : In the origin with precedence, if there are competing values for a property that are in style block matching selectors of equal specificity and scoping proximity, the last declaration in the style order is applied. 

The cascade is in ascending order, meaning: 

Animations take precedence over normal values, whether declared in user, author, or user-agent styles. 

Important values take precedence over animations, whether declared in user, author, or user-agent styles. 

Transitions take precedence over important values. 

Note: 
Transitions and animations 

Property values set by animation @keyframes are more important than all normal styles (those with no !important set). 

Property values being set in a transition take precedence over all other values set, even those marked with !important . 

The cascade algorithm is applied before the specificity algorithm, meaning if :root p { color: red;} is declared in the user stylesheet (row 2) and a less specific p {color: blue;} is in the author stylesheet (row 3), the paragraphs will be blue. 

Basic example 

Before taking a deeper look at how cascade layers impact the cascade, let's look at an example involving multiple sources of CSS across the various origins, and work through the steps of the cascade algorithm: 

Here we have a user agent stylesheet, two author stylesheets, and a user stylesheet, with no inline styles within the HTML: 

User-agent CSS: 

css 
li {
margin-left: 10px;
}

Author CSS 1: 

css 
li {
margin-left: 0;
} /* This is a reset */

Author CSS 2: 

css 
@media screen {
li {
margin-left: 3px;
}
}

@media print {
li {
margin-left: 1px;
}
}

@layer namedLayer {
li {
margin-left: 5px;
}
}

User CSS: 

css 
.specific {
margin-left: 1em;
}

HTML: 

html 
<ul>
<li class="specific">1<sup>st</sup></li>
<li>2<sup>nd</sup></li>
</ul>

In this case, declarations inside li and .specific rules should apply. 

Once again, there are five steps in the cascade algorithm, in order: 

Relevance 

Origin and importance 

Specificity 

Scoping proximity 

Order of appearance 

The 1px is for print media. Due to lack of relevance based on its media type, it is removed from consideration. 

No declaration is marked as !important , so the precedence order is author stylesheets over user stylesheets over user-agent stylesheet. Based on origin and importance , the 1em from the user stylesheet and the 10px from the user-agent stylesheet are removed from consideration. 

Note that even though the user style on .specific of 1em has a higher specificity, it's a normal declaration in a user stylesheet. As such, it has a lower precedence than any author styles, and gets removed by the origin and importance step of the algorithm before specificity even comes into play. 

There are three declarations in author stylesheets: 

css 
li {
margin-left: 0;
} /* from author css 1 */

css 
@media screen {
li {
margin-left: 3px;
}
}

css 
@layer namedLayer {
li {
margin-left: 5px;
}
}

The last one, the 5px is part of a cascade layer. Normal declarations in layers have lower precedence than normal styles not in a layer within the same origin type. This is also removed by step 2 of the algorithm, origin and importance . 

This leaves the 0 and the 3px , which both have the same selector, hence the same specificity . Neither of them are inside a @scope block, so scoping proximity does not come into play in this example either. 

We then look at order of appearance . The second one, the last of the two unlayered author styles, wins. 

css 
margin-left: 3px;

Note: 
The declaration defined in the user CSS, while it may have greater specificity, is not chosen as the cascade algorithm's origin and importance is applied before the specificity algorithm. The declaration defined in a cascade layer, though it may come later in the code, will not take precedence either as normal styles in cascade layers have less precedence than normal unlayered styles. Order of appearance only matters when both origin, importance, and specificity are equal. 

Author styles: inline styles, layers, and precedence 

The table in Cascading order provided a precedence order overview. The table summarized the user-agent, user, and author origin type styles in two lines each with "origin type - normal" and "origin type - !important". The precedence within each origin type is more nuanced. Styles can be contained within layers within their origin type, and, with author styles, there is also the issue of where inline styles land in the cascade order. 

The order in which layers are declared is important in determining precedence. Normal styles in a layer take precedence over styles declared in prior layers; with normal styles declared outside of any layer taking precedence over normal layered styles regardless of specificity. 

In this example, the author used CSS's @import rule to import five external stylesheets within a <style> information element. 

html 
<style>
@import "unlayeredStyles.css";
@import "AStyles.css" layer(A);
@import "moreUnlayeredStyles.css";
@import "BStyles.css" layer(B);
@import "CStyles.css" layer(C);
p {
color: red;
padding: 1em !important;
}
</style>

and then in the body of the document we have inline styles: 

html 
<p style="line-height: 1.6em; text-decoration: overline !important;">Hello</p>

In the CSS code block above, three cascade layers named "A", "B", and "C", were created, in that order. Three stylesheets were imported directly into layers and two were imported without creating or being assigned to a layer.
The "All unlayered styles" in the list below (normal author style precedence - order 4) includes styles from these two stylesheets and the additional unlayered CSS style blocks. In addition, there are two inline styles, a normal line-height declaration and an important text-decoration declaration: 

Precedence Order (low to high) 
Author style 
Importance 

1 
A - first layer 
normal 

2 
B - second layer 
normal 

3 
C - last layer 
normal 

4 
All unlayered styles 
normal 

5 
inline style 
normal 

6 
animations 

7 
All unlayered styles 
!important 

8 
C - last layer 
!important 

9 
B - second layer 
!important 

10 
A - first layer 
!important 

11 
inline style 
!important 

12 
transitions 

In all origin types, normal styles contained in layers have the lowest precedence. In our example, the normal styles associated with the first declared layer (A) have lower precedence than normal styles in the second declared layer (B), which have lower precedence than normal styles in the third declared layer (C). These layered styles have lower precedence than all normal unlayered styles, which includes normal styles from unlayeredStyles.css , moreUnlayeredStyles.css , and the color of p in the <style> itself. 

If any layered styles in A, B, or C have selectors with higher specificity matching an element, similar to :root body p { color: black; } , it doesn't matter. Those declarations are removed from consideration because of origin ; normal layered styles have less precedence than normal unlayered styles. If, however, the more specific selector :root body p { color: black; } was found in unlayeredStyles.css , as both origin and importance have the same precedence, specificity would mean the more specific, black declaration would win. 

The layer order of precedence is inverted for styles declared as !important . Important declarations found in a layer take precedence over important declarations found outside of a layer. Important declarations found in the first layer (A) take precedence over important declarations found in layer B, which take precedence over important declarations found in layer C, which take precedence over important declarations found outside of a layer. 

Inline styles 

Only relevant to author styles are inline styles, declared with the style attribute. Normal inline styles take precedence over any other normal author styles, no matter the specificity of the selector. If line-height: 2; were declared in a :root body p selector block in any of the five imported stylesheets, the line height would still be 1.6 . Normal inline styles do not take precedence over animated or transitioned properties. 

Important inline styles take precedence over all other author styles, regardless of whether they are important, inline, or layered. Important inline styles also take precedence over animated properties, but not transitioned properties. Three things can override an important inline style: 

An important user style. 

An important user agent style. 

A transitioned property. 

Importance and layers 

The origin type precedence order is inverted for important styles. Important styles declared outside of any cascade layer have lower precedence than those declared as part of a layer. Important styles that come in early layers take precedence over important styles declared in subsequent cascade layers. 

Take for example the following CSS: 

css 
p {
color: red;
}

@layer B {
:root p {
color: blue;
}
}

Even though the red is declared first and has a less specific selector, because unlayered CSS takes precedence over layered CSS, the paragraph will be red. Had we included an inline style on the paragraph setting it to a different color, such as <p style="color: black"> , the paragraph would be black. 

When we add !important to this bit of CSS, the precedence order is reversed with the stylesheet: 

css 
p {
color: red !important;
}

@layer B {
:root p {
color: blue !important;
}
}

Now the paragraph will be blue. The !important in the earliest declared layer takes precedence over subsequent layers and unlayered important declarations. If the inline style contained !important , such as <p style="color: black !important"> , again the paragraph would be black. Inline importance does take precedence over all other author declared !important declarations, no matter the specificity. 

Note: 
The !important flag reverses the precedence of cascade layers. For this reason, try not to use !important to override external styles. Instead, use @import together with the layer keyword or layer() function to import external stylesheets (from frameworks, widget stylesheets, libraries, etc.) into layers. Importing stylesheets into a layer as the first declaration in your CSS demotes their precedence, and author-defined layers, defined later in your CSS, will have higher precedence. The !important flag should only be used sparingly, if ever, to guard required styles against later overrides, in the first declared layer. 

Styles that are transitioning take precedence over all important styles, no matter who or how they are declared. 

Complete cascade order 

Now that we have a better understanding of origin type and cascade layer precedence, we realize the table in Cascading order could have more accurately been represented by the following table: 

Precedence Order 
(low to high) Style Origin Importance 

1 user-agent - first declared layer normal 

user-agent - last declared layer 

user-agent - unlayered styles 

2 user - first declared layer normal 

user - last declared layer 

user - unlayered styles 

3 author - first declared layer normal 

author - last declared layer 

author - unlayered styles 

inline style 

4 animations 

5 author - unlayered styles !important 

author - last declared layer 

author - first declared layer 

inline style 

6 user - unlayered styles !important 

user - last declared layer 

user - first declared layer 

7 user-agent - unlayered styles !important 

user-agent - last declared layer 

user-agent - first declared layer 

8 transitions 

Which CSS entities participate in the cascade 

Only CSS property/value pair declarations participate in the cascade. CSS at-rule descriptors don't participate in the cascade and HTML presentational attributes are not part of the cascade. 

At-rules 

CSS at-rules containing entities other than declarations, such as a @font-face rule containing descriptors , don't participate in the cascade. 

For the most part, the properties and descriptors defined in at-rules don't participate in the cascade. Only at-rules as a whole participate in the cascade. For example, within a @font-face rule, font names are identified by font-family descriptors. If several @font-face rules with the same descriptor are defined, only the most appropriate @font-face , as a whole, is considered. If more than one are identically appropriate, the entire @font-face declarations are compared using steps 1, 2, and 4 of the algorithm (there is no specificity when it comes to at-rules). 

While the declarations contained in most at-rules — such as those in @media , @document , or @supports — participate in the cascade, the at-rule may make an entire selector not relevant, as we saw with the print style in the basic example . 

Declarations in @keyframes don't participate in the cascade. As with @font-face , only the @keyframes as a whole is selected via the cascade algorithm. The precedence order of animation is described below . 

When it comes to @import , the @import doesn't participate itself in the cascade, but all of the imported styles do participate. If the @import defines a named or anonymous layer , the contents of the imported stylesheet are placed into the specified layer. All other CSS imported with @import is treated as the last declared layer. This was discussed above. 

Finally, @charset obeys specific algorithms and isn't affected by the cascade algorithm. 

Presentational attributes 

Presentational attributes are attributes in the source document that can affect styling. For example, when included, the deprecated align attribute sets the alignment on several HTML elements and the fill attribute defines the color used to paint SVG shapes and text and defines the final state for SVG animations. While they are author styles, presentational attributes do not participate in the cascade. 

If the HTML presentation attribute is supported by the user agent, valid presentational attributes included in HTML and SVG, such as the align or fill attributes, are translated to the corresponding CSS rules (all SVG presentation attributes are supported as CSS properties) and inserted in the author stylesheet prior to any other styles with a specificity equal to 0 . 

Presentational attributes cannot be declared !important . 

CSS animations and the cascade 

CSS animations , using @keyframes at-rules, define animations between states. @keyframes don't cascade, meaning that at any given time CSS takes values from only one single set of @keyframes and never mixes multiple ones. If multiple sets of @keyframes are defined with the same animation name, the last defined set in the origin and layer with the greatest precedence is used. Other @keyframes are ignored, even if they animate different properties. 

css 
p {
animation: infinite 5s alternate repeatedName;
}

@keyframes repeatedName {
from {
font-size: 1rem;
}
to {
font-size: 3rem;
}
}

@layer A {
@keyframes repeatedName {
from {
background-color: yellow;
}
to {
background-color: orange;
}
}
}

@layer B {
@keyframes repeatedName {
from {
color: white;
}
to {
color: black;
}
}
}

In this example, there are three separate animation declaration named repeatedName . When animation: infinite 5s alternate repeatedName is applied to the paragraph, only one animation is applied: the keyframe animation defined in the unlayered CSS takes precedence over the layered keyframe animation declarations based on origin and layer precedence order. In this example, only the element's font size will be animated. 

Note: 
There are no important animations, as property declarations in a @keyframes block that contain !important as part of the value are ignored. 

Resetting styles 

After your content has finished altering styles, it may find itself in a situation where it needs to restore them to a known state. This may happen in cases of animations, theme changes, and so forth. The CSS property all lets you quickly set (almost) everything in CSS back to a known state. 

all lets you opt to immediately restore all properties to any of their initial (default) state, the state inherited from the previous level of the cascade, a specific origin (the user-agent stylesheet, the author stylesheet, or the user stylesheet), or even to clear the values of the properties entirely. 

Specifications 

Specification 

CSS Cascading and Inheritance Level 4 

See also 

CSS cascading and inheritance module 

Learn: Handling conflicts 

Learn: Cascade layers 

CSS syntax 

Specificity 

Inheritance 

At-rules 

Values: initial , computed , used , and actual 

Help improve MDN

Was this page helpful to you? 

Yes 

No 

Learn how to contribute 
This page was last modified on Dec 16, 2025 by MDN contributors .

View this page on GitHub • Report a problem with this content