import summaryFixture from '../mirage/fixtures/summary';

export default function() {
    this.get('/summary', () => summaryFixture);
}
