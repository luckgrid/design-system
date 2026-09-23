---
title: Harbour tides and reading the water
linkTitle: Article
weight: 20
description: Long-form classless content on the semantic base, in an explicit dark scheme.
scheme: dark
---

Tide tables reward patient reading. This page uses ordinary HTML elements only,
with no class, so the classless base carries the whole presentation. The
[first section](#reading-the-table) links inside a sentence, and **strong**,
<b>bold</b>, <small>small</small>, and <mark>marked</mark> text sit inline.

## Reading the table

Press <kbd>Tab</kbd> to move focus. The tool printed <samp>ok</samp>, and the
stylesheet sets `color-scheme`. A long unbroken word and address must wrap
instead of widening the page: Pneumonoultramicroscopicsilicovolcanoconiosispneumonoultramicroscopicsilicovolcanoconiosis and
[#a-very-long-fragment-identifier-that-keeps-going-without-any-natural-break-point-at-all](#a-very-long-fragment-identifier-that-keeps-going-without-any-natural-break-point-at-all).

<p dir="rtl" lang="ar">نص من اليمين إلى اليسار مع <a href="#reading-the-table">رابط</a> داخل الجملة.</p>

### Heading level three

#### Heading level four

##### Heading level five

###### Heading level six

> A quotation, set apart from the text around it.

---

## Lists

- An unordered item
- Another unordered item

1. The first step
2. The second step

<dl>
  <dt>Term</dt>
  <dd>Its description.</dd>
</dl>

## Code and media

<pre tabindex="0"><code>@layer app;
@layer app { a { text-decoration-thickness: 2px; } } /* a long line that must scroll inside the block instead of widening the page */</code></pre>

<figure>
  <img src="/figure.svg" alt="Three bars of increasing height" width="480" height="160">
  <figcaption>An image figure with a caption.</figcaption>
</figure>

<p>An inline <svg role="img" aria-label="Circle" width="16" height="16" viewBox="0 0 16 16"><circle cx="8" cy="8" r="7" fill="currentColor"></circle></svg> icon.</p>

## Table

<table>
  <caption>Release floor by engine</caption>
  <thead>
    <tr>
      <th scope="col">Engine</th>
      <th scope="col">Minimum</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <th scope="row">Chromium</th>
      <td>123</td>
    </tr>
    <tr>
      <th scope="row">Firefox</th>
      <td>121</td>
    </tr>
  </tbody>
</table>

## Form

<form action="#form-sent">
  <fieldset>
    <legend>Contact</legend>
    <p>
      <label for="name">Name</label>
      <input id="name" name="name" type="text" required autocomplete="name">
    </p>
    <p>
      <label for="email">Email</label>
      <input id="email" name="email" type="email" placeholder="you@example.test" autocomplete="email">
    </p>
    <p>
      <label for="topic">Topic</label>
      <select id="topic" name="topic">
        <option>Question</option>
        <option>Feedback</option>
        <option>Other</option>
      </select>
    </p>
    <p>
      <label for="message">Message</label>
      <textarea id="message" name="message" rows="3"></textarea>
    </p>
    <p>
      <input id="updates" name="updates" type="checkbox">
      <label for="updates">Send updates</label>
    </p>
    <p>
      <input id="reply-email" name="reply" type="radio" value="email" checked>
      <label for="reply-email">Reply by email</label>
      <input id="reply-none" name="reply" type="radio" value="none">
      <label for="reply-none">No reply</label>
    </p>
    <p>
      <label for="disabled">Disabled</label>
      <input id="disabled" name="disabled" type="text" value="Not editable" disabled>
    </p>
  </fieldset>
  <button type="submit">Send</button>
  <button type="button">Native button</button>
  <input type="reset" value="Reset">
</form>

## Disclosure

<details>
  <summary>More detail</summary>
  <p>Content revealed by the native disclosure.</p>
</details>

<button type="button" popovertarget="note">Show note</button>
<div id="note" popover>
  <p>A popover note. Where Popover is unsupported, it stays visible here.</p>
</div>

<dialog id="dialog" aria-labelledby="dialog-title" open>
  <h2 id="dialog-title">Dialog</h2>
  <p>A native dialog.</p>
  <form method="dialog">
    <button>Close</button>
  </form>
</dialog>
