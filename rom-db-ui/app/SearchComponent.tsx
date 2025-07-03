'use client';

import { useState } from 'react';

type FolderData = {
    folder: string;
    images: string[];
};

export default function SearchComponent({ folderData }: { folderData: FolderData[] }) {
    const [searchTerm, setSearchTerm] = useState('');

    const filteredFolders = folderData.filter(item =>
        item.folder.toLowerCase().includes(searchTerm.toLowerCase())
    );

    return (
        <>
            <div className="mb-4 sm:mb-6">
                <input
                    type="text"
                    placeholder="Search screenshots..."
                    className="w-full p-2 sm:p-3 border rounded text-sm sm:text-base"
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                />
            </div>

            {filteredFolders.length === 0 ? (
                <p className="text-sm sm:text-base">No matching titles found.</p>
            ) : (
                <div>
                    {filteredFolders.map(({ folder, images }) => (
                        <div key={folder} className="mb-4 sm:mb-6">
                            <h2 className="text-lg sm:text-xl font-bold mb-2 px-1">{folder}</h2>

                            {images.length === 0 ? (
                                <p className="text-gray-500 text-sm sm:text-base px-1">No images found for this title</p>
                            ) : (
                                <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-2 sm:gap-4">
                                    {images.map((image) => (
                                        <div key={image} className="border rounded overflow-hidden">
                                            <img
                                                src={`/screenshots/${encodeURIComponent(folder)}/${encodeURIComponent(image)}`}
                                                alt={`Screenshot from ${folder}`}
                                                className="w-full h-32 sm:h-40 md:h-48 object-contain"
                                                loading="lazy"
                                            />
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </>
    );
}
