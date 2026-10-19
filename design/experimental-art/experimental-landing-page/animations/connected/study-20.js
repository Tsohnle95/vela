export default {
 name:'Caustic curl',family:'Surface',description:'A faint curved caustic bends inside each cube toward the passing pearl.',rule:'The curl turns with the pearl’s bearing and grows slightly as it approaches.',
 setup({path,faceEffects,set}){const nodes=faceEffects.map(g=>path(g,'M-90 25C-65 -55 70 -65 85 5C95 55 -30 70 -45 10','#e8dcff',5));return ({i,x,y,angle,strength})=>set(nodes[i],{transform:`translate(${x} ${y}) rotate(${angle*180/Math.PI}) scale(${.65+strength*.3})`,opacity:strength*.29});}
};
