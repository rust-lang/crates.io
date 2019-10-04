import { module, test } from 'qunit';
import { setupTest } from 'ember-qunit';
import { A } from '@ember/array';
import Service from '@ember/service';

module('Unit | Controller | crate/version', function(hooks) {
  setupTest(hooks);
  const userId = 1;
  // stub the session service
  // https://guides.emberjs.com/release/testing/testing-components/#toc_stubbing-services
  const sessionStub = Service.extend();

  hooks.beforeEach(function() {
    sessionStub.currentUser = { id: userId };
    this.owner.register('service:session', sessionStub);
  });

  test('notYankedOrIsOwner is true when conditions fulfilled', function(assert) {
    assert.expect(2);
    let controller = this.owner.lookup('controller:crate/version');
    controller.model = { yanked: false };
    controller.crate = { owner_user: A([{ id: userId }]) };
    assert.ok(controller);
    assert.ok(controller.notYankedOrIsOwner);
  });

  test('notYankedOrIsOwner is false when conditions fulfilled', function(assert) {
    assert.expect(2);
    let controller = this.owner.lookup('controller:crate/version');
    controller.model = { yanked: true };
    controller.crate = { owner_user: A([{ id: userId }]) };
    assert.ok(controller);
    assert.notOk(controller.notYankedOrIsOwner);
  });
});
