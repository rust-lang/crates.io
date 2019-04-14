import { helper } from '@ember/component/helper';

export default helper(function(params) {
    const [value] = params;
    if (!value) {
        return value;
    }
    if (value.length > 200) {
        return `${value.slice(0, 200)} ...`;
    }
    return value;
});
