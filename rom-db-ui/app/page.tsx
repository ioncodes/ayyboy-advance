import fs from 'fs';
import path from 'path';
import SearchComponent from './SearchComponent';

function getScreenshotFolders() {
    try {
        const screenshotsPath = path.resolve(process.cwd(), 'public/screenshots/');

        if (!fs.existsSync(screenshotsPath)) {
            console.error('Screenshots directory not found');
            return [];
        }

        const entries = fs.readdirSync(screenshotsPath, { withFileTypes: true });

        return entries
            .filter(entry => entry.isDirectory())
            .map(dir => dir.name);

    } catch (error) {
        console.error('Error fetching screenshot folders:', error);
        return [];
    }
}

function getImagesInFolder(folderName: string) {
    try {
        const folderPath = path.resolve(process.cwd(), 'public/screenshots/', folderName);
        const entries = fs.readdirSync(folderPath);

        // only include .png files
        const filteredEntries = entries.filter(file => file.toLowerCase().endsWith('.png'));

        // sort by name_<NUMBER>
        filteredEntries.sort((a, b) => {
            const aNumber = parseInt(a.match(/_(\d+)\.png$/)?.[1] || '0', 10);
            const bNumber = parseInt(b.match(/_(\d+)\.png$/)?.[1] || '0', 10);
            return aNumber - bNumber;
        });

        return filteredEntries;
    } catch (error) {
        console.error(`Error reading images from folder ${folderName}:`, error);
        return [];
    }
}

export default function ScreenshotsPage() {
    const folders = getScreenshotFolders();

    const folderImages = folders.map(folder => {
        const images = getImagesInFolder(folder);
        return { folder, images };
    });

    return (
        <div className="space-y-8 p-4">
            <SearchComponent folderData={folderImages} />
        </div>
    );
}
