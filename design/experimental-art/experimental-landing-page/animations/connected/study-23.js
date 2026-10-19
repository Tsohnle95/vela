export default {
 name:'Elastic response',family:'Physics',description:'Each cube stretches a fraction toward the pearl while gently compressing across it.',rule:'A directional stretch of at most 1.8 percent follows the pearl, with no bounce timer.',
 setup({cubeGroups,set}){return ({i,x,y,angle,strength})=>{const a=angle*180/Math.PI;set(cubeGroups[i],{transform:`translate(${x} ${y}) rotate(${a}) scale(${1+strength*.018} ${1-strength*.009}) rotate(${-a}) translate(${-x} ${-y})`});};}
};
