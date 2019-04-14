import { helper } from '@ember/component/helper';

export default helper(function(params) {
    const [req] = params;
    return req === '*' ? '' : req;
});
