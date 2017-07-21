import DS from 'ember-data';
import { validator, buildValidations } from 'ember-cp-validations';

const Validations = buildValidations({
    email: {
        validators: [
            validator('presence', true),
            validator('format', {
                type: 'email'
            }),
            validator('length', {
                max: 200,
            })
        ]
    },
});

export default DS.Model.extend(Validations, {
    email: DS.attr('string'),
    name: DS.attr('string'),
    login: DS.attr('string'),
    avatar: DS.attr('string'),
    url: DS.attr('string'),
    kind: DS.attr('string'),

    stats() {
        return this.store.adapterFor('user').stats(this.get('id'));
    },
});
