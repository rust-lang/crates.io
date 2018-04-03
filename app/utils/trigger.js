export default function trigger(target, eventName) {
    let event = document.createEvent('Event');
    event.initEvent(eventName, true, true);
    target.dispatchEvent(event);
}
