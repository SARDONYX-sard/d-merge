/** vitest-environment browser */
// SPDX-License-Identifier: MIT or Apache-2.0
// Copyright (c) 2022 Takefumi Yoshii
//
// # References
// - https://github.com/takefumi-yoshii/nextjs-testing-strategy-2022/blob/main/src/tests/jest.customMatchers.test.tsx
import '@testing-library/jest-dom';
import { render } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

describe('Template', () => {
  test('<main>(role=main)', () => {
    const { container } = render(
      <main>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </main>,
    );
    expect(container).not.toBeAtom();
    expect(container).not.toBeMolecule();
    expect(container).not.toBeOrganism();
    expect(container).toBeTemplate();
  });
});

describe('Organisms', () => {
  function asserts(container: HTMLElement) {
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeAtom();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeMolecule();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).toBeOrganism();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeTemplate();
  }
  test('<nav>(role=navigation)', () => {
    const { container } = render(
      <nav>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </nav>,
    );
    asserts(container);
  });
  test('<aside>(role=complementary)', () => {
    const { container } = render(
      <aside>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </aside>,
    );
    asserts(container);
  });
  test('<form>(role=form)', () => {
    const { container } = render(
      <form aria-labelledby='test'>
        <h2 id='test'>Test</h2>
        <button type='button'>+1</button>
      </form>,
    );
    asserts(container);
  });
  test('<section>(role=region)', () => {
    const { container } = render(
      <section aria-labelledby='test'>
        <h2 id='test'>Test</h2>
        <button type='button'>+1</button>
      </section>,
    );
    asserts(container);
  });
  test('<div>(role=alertdialog)', () => {
    const { container } = render(
      <div aria-label='test' role='alertdialog'>
        <h2 id='test'>Test</h2>
        <button type='button'>+1</button>
      </div>,
    );
    asserts(container);
  });
  test('<form>(role=search)', () => {
    const { container } = render(
      // biome-ignore lint/a11y/useSemanticElements: <explanation>
      <form role='search'>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </form>,
    );
    asserts(container);
  });
});

describe('Molecules', () => {
  function asserts(container: HTMLElement) {
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeAtom();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).toBeMolecule();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeOrganism();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeTemplate();
  }
  test('<form>(role=none)', () => {
    const { container } = render(
      <form>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </form>,
    );
    asserts(container);
  });
  test('<div>(role=group)', () => {
    const { container } = render(
      // biome-ignore lint/a11y/useSemanticElements: <explanation>
      <div role='group'>
        <h2>Test</h2>
      </div>,
    );
    asserts(container);
  });
  test('<article>(role=artilce)', () => {
    const { container } = render(
      <article>
        <h2>Test</h2>
      </article>,
    );
    asserts(container);
  });
  test('<ul>(role=list)', () => {
    const { container } = render(
      <ul>
        <li />
      </ul>,
    );
    asserts(container);
  });
  test('<ol>(role=list)', () => {
    const { container } = render(
      <ol>
        <li />
      </ol>,
    );
    asserts(container);
  });
  test('<dl>(role=term)', () => {
    const { container } = render(
      <dl>
        <dt />
        <dd />
      </dl>,
    );
    asserts(container);
  });
  test('<div>(role=tablist)', () => {
    const { container } = render(
      <div role='tablist'>
        {/* biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: <explanation> */}
        <p role='tab' tabIndex={0} />
      </div>,
    );
    asserts(container);
  });
  test('<div>(role=tabpanel)', () => {
    const { container } = render(
      <div role='tabpanel'>
        <h2>Test</h2>
      </div>,
    );
    asserts(container);
  });
  test('<table>(role=table)', () => {
    const { container } = render(
      <table>
        <thead>
          <tr />
        </thead>
        <tbody>
          <tr />
        </tbody>
      </table>,
    );
    asserts(container);
  });
  test('<select>(role=combobox)', () => {
    const { container } = render(
      <select>
        <option value={0}>A</option>
        <option value={1}>B</option>
      </select>,
    );
    asserts(container);
  });
  test('<section>(multiple roles)', () => {
    const { container } = render(
      <section>
        <h2 id='test'>Test</h2>
        <button type='button'>+1</button>
      </section>,
    );
    asserts(container);
  });
});

describe('Organisms or Molecules', () => {
  // MEMO: header footer は構成 Node によって AsNonLandmark になりえるため
  function asserts(container: HTMLElement) {
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeAtom();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).toBeMolecule();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).toBeOrganism();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeTemplate();
  }
  test('<header>(role=banner)', () => {
    const { container } = render(
      <header>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </header>,
    );
    asserts(container);
  });
  test('<footer>(role=contentinfo)', () => {
    const { container } = render(
      <footer>
        <h2>Test</h2>
        <button type='button'>+1</button>
      </footer>,
    );
    asserts(container);
  });
});

describe('Atoms', () => {
  function asserts(container: HTMLElement) {
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).toBeAtom();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeMolecule();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeOrganism();
    // biome-ignore lint/suspicious/noMisplacedAssertion: <explanation>
    expect(container).not.toBeTemplate();
  }
  test('<dialog>(role=dialog)', () => {
    const { container } = render(
      <dialog>
        <h2 id='test'>Test</h2>
        <button type='button'>+1</button>
      </dialog>,
    );
    asserts(container);
  });
  test('<p>(role=none)', () => {
    const { container } = render(<p>test</p>);
    asserts(container);
  });
  test('<h1>(role=heading)', () => {
    const { container } = render(<h1>test</h1>);
    asserts(container);
  });
  test('<a>(role=link)', () => {
    // biome-ignore lint/a11y/useValidAnchor: <explanation>
    const { container } = render(<a href='#'>test</a>);
    asserts(container);
  });
  test('<button(role=button)', () => {
    const { container } = render(<button type='button'>test</button>);
    asserts(container);
  });
  test('<input>(role=textbox)', () => {
    const { container } = render(<input type='text' />);
    asserts(container);
  });
  test('<textarea>(role=textbox)', () => {
    const { container } = render(<textarea />);
    asserts(container);
  });
  test('<div>(single role)', () => {
    const { container } = render(
      <div>
        <p>test</p>
      </div>,
    );
    asserts(container);
  });
  test('<label>(role=none)', () => {
    const { container } = render(
      <label>
        <input id='check' type='checkbox' />
        Test
      </label>,
    );
    const { container: htmlFor } = render(
      <>
        <label htmlFor='check'>Test</label>
        <input id='check' type='checkbox' />
      </>,
    );
    asserts(container);
    asserts(htmlFor);
  });
  test('text(role=none)', () => {
    const { container } = render(
      <div>
        <p>test</p>
      </div>,
    );
    asserts(container);
  });
});
