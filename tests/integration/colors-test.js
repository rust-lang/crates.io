import { render } from '@ember/test-helpers';
import { setupRenderingTest } from 'ember-qunit';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';
import { percySnapshot } from 'ember-percy';

module('Color Scheme', function (hooks) {
  setupRenderingTest(hooks);

  test('is looking good', async function (assert) {
    assert.expect(0);

    await render(hbs`
      <style>
        .box {
          height: 80px;
          width: 80px;
          border-radius: 5px;
          box-shadow: inset 0 1px 2px 0 hsla(0, 0%, 0%, .3);
        }
      </style>
      <table style="border-spacing: 10px">
        <thead>
          <tr>
            <th></th>
            <th>999</th>
            <th>900</th>
            <th>800</th>
            <th>700</th>
            <th>600</th>
            <th>500</th>
            <th>400</th>
            <th>300</th>
            <th>200</th>
            <th>100</th>
            <th>000</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th>Grey</th>
            <td class="box" style="background: var(--grey999)"></td>
            <td class="box" style="background: var(--grey900)"></td>
            <td class="box" style="background: var(--grey800)"></td>
            <td class="box" style="background: var(--grey700)"></td>
            <td class="box" style="background: var(--grey600)"></td>
            <td class="box" style="background: var(--grey500)"></td>
            <td class="box" style="background: var(--grey400)"></td>
            <td class="box" style="background: var(--grey300)"></td>
            <td class="box" style="background: var(--grey200)"></td>
            <td class="box" style="background: var(--grey100)"></td>
            <td class="box" style="background: var(--grey000)"></td>
          </tr>
          <tr>
            <th>Yellow</th>
            <td></td>
            <td class="box" style="background: var(--yellow900)"></td>
            <td class="box" style="background: var(--yellow800)"></td>
            <td class="box" style="background: var(--yellow700)"></td>
            <td class="box" style="background: var(--yellow600)"></td>
            <td class="box" style="background: var(--yellow500)"></td>
            <td class="box" style="background: var(--yellow400)"></td>
            <td class="box" style="background: var(--yellow300)"></td>
            <td class="box" style="background: var(--yellow200)"></td>
            <td class="box" style="background: var(--yellow100)"></td>
            <td></td>
          </tr>
          <tr>
            <th>Green</th>
            <td></td>
            <td class="box" style="background: var(--green900)"></td>
            <td class="box" style="background: var(--green800)"></td>
            <td class="box" style="background: var(--green700)"></td>
            <td class="box" style="background: var(--green600)"></td>
            <td class="box" style="background: var(--green500)"></td>
            <td class="box" style="background: var(--green400)"></td>
            <td class="box" style="background: var(--green300)"></td>
            <td class="box" style="background: var(--green200)"></td>
            <td class="box" style="background: var(--green100)"></td>
            <td></td>
          </tr>
          <tr>
            <th>Violet</th>
            <td></td>
            <td class="box" style="background: var(--violet900)"></td>
            <td class="box" style="background: var(--violet800)"></td>
            <td class="box" style="background: var(--violet700)"></td>
            <td class="box" style="background: var(--violet600)"></td>
            <td class="box" style="background: var(--violet500)"></td>
            <td class="box" style="background: var(--violet400)"></td>
            <td class="box" style="background: var(--violet300)"></td>
            <td class="box" style="background: var(--violet200)"></td>
            <td class="box" style="background: var(--violet100)"></td>
            <td></td>
          </tr>
        </tbody>
      </table>
    `);

    await percySnapshot(assert);
  });
});
